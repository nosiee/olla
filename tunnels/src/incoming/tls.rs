use bytes::BytesMut;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, WriteHalf};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::{net::TcpStream, sync::mpsc::Sender};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;
use tracing::debug;

use crate::header;
use crate::{AsyncIncomingTunnel, errors::TunnelError};
use device::Message;

pub struct TLSTunnel {
    addr: SocketAddr,
    cert: String,
    key: String,

    peers: RwLock<HashMap<String, WriteHalf<TlsStream<TcpStream>>>>,
}

impl AsyncIncomingTunnel for TLSTunnel {
    async fn forward(self: Arc<Self>, tx: Sender<Message>) -> anyhow::Result<(), TunnelError> {
        let certs = CertificateDer::pem_file_iter(&self.cert).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        let key = PrivateKeyDer::from_pem_file(&self.key).unwrap();
        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .unwrap();

        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind(&self.addr).await.unwrap();
        let self_c = self.clone();

        tokio::spawn(async move {
            loop {
                let (stream, peer) = listener.accept().await.unwrap();
                let stream = acceptor.accept(stream).await.unwrap();
                self_c.addr_peer(&peer, stream, tx.clone()).await;

                debug!("{} new peer connected", peer.to_string());
            }
        });

        Ok(())
    }

    async fn write(&self, peer: String, payload: &[u8]) -> anyhow::Result<usize, TunnelError> {
        let mut peers_guard = self.peers.write().await;

        if let Some(stream) = peers_guard.get_mut(&peer) {
            let payload = header::extend_payload(payload);

            if let Ok(n) = stream.write(&payload).await {
                debug!("{} bytes written to {}", n, peer);
                return Ok(n);
            }
        }

        Ok(0)
    }
}

impl TLSTunnel {
    pub fn new(addr: SocketAddr, cert: String, key: String) -> Self {
        Self {
            addr,
            cert,
            key,

            peers: RwLock::new(HashMap::new()),
        }
    }

    async fn addr_peer(&self, peer: &SocketAddr, stream: TlsStream<TcpStream>, tx: Sender<Message>) {
        let (mut r_stream, w_stream) = tokio::io::split(stream);
        let mut peers_guard = self.peers.write().await;

        let peer_str = peer.to_string();
        peers_guard.insert(peer_str.clone(), w_stream);

        tokio::spawn(async move {
            loop {
                let mut header_buf = [0; header::HEADER_SIZE];

                if r_stream.read_exact(&mut header_buf).await.is_ok() {
                    let header_frame = header::decode(header_buf);
                    let payload_size: usize = header_frame.frame_size as usize - header::HEADER_SIZE;
                    debug!("peer {} header frame {:?}", peer_str.clone(), &header_buf);

                    if header_frame.frame_size > 0 {
                        let mut buf = BytesMut::zeroed(payload_size);

                        if let Ok(n) = r_stream.read_exact(&mut buf).await {
                            buf.truncate(n);

                            debug!("read {} bytes from {}", n + header::HEADER_SIZE, peer_str.clone());
                            if tx.send(buf.freeze()).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });
    }
}
