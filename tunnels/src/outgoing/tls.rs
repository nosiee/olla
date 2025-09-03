use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use std::mem;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

use crate::AsyncOutgoingTunnel;
use crate::errors::{CONNECT_ERROR, DEFAULT_ERROR_CODE, SNI_PARSING_ERROR, TLS_CONNECT_ERROR, TunnelError};
use crate::header;

#[derive(Debug)]
pub struct TLSTunnel {
    w_stream: RwLock<Option<WriteHalf<TlsStream<TcpStream>>>>,
    r_stream: RwLock<Option<ReadHalf<TlsStream<TcpStream>>>>,

    addr: Option<SocketAddr>,
    ca: Option<String>,
    sni: Option<String>,

    session_expired_at: RwLock<Option<SystemTime>>,
    session_ttl: Duration,
    keepalive: bool,
    keepwarm: bool,

    prevent_tot: bool,
}

impl AsyncOutgoingTunnel for TLSTunnel {
    async fn send(&self, mut payload: Vec<u8>) -> anyhow::Result<usize, TunnelError> {
        if self.w_stream.read().await.is_none() {
            let conn = self.connect().await?;
            let (r_stream, w_stream) = tokio::io::split(conn);

            let mut w_stream_guard = self.w_stream.write().await;
            let mut r_stream_guard = self.r_stream.write().await;

            *w_stream_guard = Some(w_stream);
            *r_stream_guard = Some(r_stream);

            if self.keepwarm || !self.session_ttl.is_zero() {
                let mut session_guard = self.session_expired_at.write().await;
                *session_guard = Some(SystemTime::now().checked_add(self.session_ttl).unwrap());
            }
        }

        let mut guard = self.w_stream.write().await;
        let stream = guard.as_mut().unwrap();
        header::add(&mut payload);

        if let Err(err) = stream.write_all(payload.as_slice()).await {
            return Err(TunnelError::IO((err.to_string(), err.raw_os_error().unwrap_or(DEFAULT_ERROR_CODE))));
        }

        if self.keepwarm {
            let mut session_guard = self.session_expired_at.write().await;
            *session_guard = Some(SystemTime::now().checked_add(self.session_ttl).unwrap());
        }

        Ok(payload.len())
    }

    async fn recv(&self, buffer: &mut [u8]) -> anyhow::Result<usize, TunnelError> {
        let mut guard = self.r_stream.write().await;
        let stream = guard.as_mut().unwrap();

        match stream.read(buffer).await {
            Ok(n) => Ok(n),
            Err(err) => Err(TunnelError::IO((err.to_string(), err.raw_os_error().unwrap_or(DEFAULT_ERROR_CODE)))),
        }
    }

    async fn recv_exact(&self, buffer: &mut [u8]) -> anyhow::Result<usize, TunnelError> {
        let mut guard = self.r_stream.write().await;
        let stream = guard.as_mut().unwrap();

        match stream.read_exact(buffer).await {
            Ok(n) => Ok(n),
            Err(err) => Err(TunnelError::IO((err.to_string(), err.raw_os_error().unwrap_or(DEFAULT_ERROR_CODE)))),
        }
    }

    async fn check_connect(&self) -> anyhow::Result<(), TunnelError> {
        let _ = self.connect().await?;

        // NOTE(nosiee): to prevent the UnexpectedEof error on the server side, we need to close the connection properly
        // not really that important in the check_connect call, but keeps the log clean
        self.shutdown().await
    }
}

impl Default for TLSTunnel {
    fn default() -> Self {
        Self::new()
    }
}

impl TLSTunnel {
    pub fn new() -> Self {
        Self {
            w_stream: RwLock::new(None),
            r_stream: RwLock::new(None),

            addr: None,
            ca: None,
            sni: None,

            session_expired_at: RwLock::new(None),
            session_ttl: Duration::ZERO,
            keepalive: false,
            keepwarm: false,

            prevent_tot: false,
        }
    }

    pub fn set_addr(mut self, addr: SocketAddr) -> Self {
        self.addr = Some(addr);
        self
    }

    pub fn set_session_ttl(mut self, ttl: Duration) -> Self {
        if !ttl.is_zero() {
            self = self.set_keepalive(false);
        }

        self.session_ttl = ttl;
        self
    }

    pub fn set_keepalive(mut self, keepalive: bool) -> Self {
        if keepalive {
            self = self.set_session_ttl(Duration::ZERO).set_keepwarm(false);
        }

        self.keepalive = keepalive;
        self
    }

    pub fn set_keepwarm(mut self, keepwarm: bool) -> Self {
        if keepwarm {
            self = self.set_keepalive(false);
        }

        self.keepwarm = keepwarm;
        self
    }

    pub fn set_prevent_tot(mut self, prevent_tot: bool) -> Self {
        self.prevent_tot = prevent_tot;
        self
    }

    pub fn set_ca(mut self, ca: String) -> Self {
        self.ca = Some(ca);
        self
    }

    pub fn set_sni(mut self, sni: String) -> Self {
        self.sni = Some(sni);
        self
    }

    pub async fn shutdown(&self) -> anyhow::Result<(), TunnelError> {
        let w_stream = mem::take(&mut *self.w_stream.write().await);
        let r_stream = mem::take(&mut *self.r_stream.write().await);

        if w_stream.is_none() && r_stream.is_some() {
            let mut conn = ReadHalf::unsplit(r_stream.unwrap(), w_stream.unwrap());

            if let Err(err) = conn.shutdown().await {
                return Err(TunnelError::Connection((
                    err.to_string(),
                    err.raw_os_error().unwrap_or(DEFAULT_ERROR_CODE),
                )));
            }
        }

        Ok(())
    }

    async fn connect(&self) -> anyhow::Result<TlsStream<TcpStream>, TunnelError> {
        let ca = self.ca.as_ref().unwrap().clone();
        let sni = self.sni.as_ref().unwrap().clone();
        let addr = self.addr.to_owned().unwrap();

        let mut root_cert_store = rustls::RootCertStore::empty();
        for cert in CertificateDer::pem_file_iter(ca).unwrap() {
            root_cert_store.add(cert.unwrap()).unwrap();
        }

        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(config));

        let stream = match TcpStream::connect(addr).await {
            Ok(stream) => stream,
            Err(err) => {
                return Err(TunnelError::Connection((err.to_string(), err.raw_os_error().unwrap_or(CONNECT_ERROR))));
            }
        };

        let domain = match ServerName::try_from(sni) {
            Ok(domain) => domain,
            Err(err) => {
                return Err(TunnelError::Connection((err.to_string(), SNI_PARSING_ERROR)));
            }
        };

        match connector.connect(domain, stream).await {
            Ok(tls_stream) => Ok(tls_stream),
            Err(err) => Err(TunnelError::Connection((err.to_string(), TLS_CONNECT_ERROR))),
        }
    }
}
