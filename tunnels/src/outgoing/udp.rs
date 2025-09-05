use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tracing::debug;

use crate::AsyncOutgoingTunnel;
use crate::errors::{CONNECT_ERROR, DEFAULT_ERROR_CODE, TunnelError};

#[derive(Debug)]
pub struct UDPTunnel {
    socket: RwLock<Option<UdpSocket>>,
    addr: Option<SocketAddr>,

    session_expired_at: RwLock<Option<SystemTime>>,
    session_ttl: Duration,
    keepalive: bool,
    keepwarm: bool,
}

impl AsyncOutgoingTunnel for UDPTunnel {
    async fn send(&self, payload: &[u8]) -> anyhow::Result<usize, TunnelError> {
        if self.socket.read().await.is_none() {
            let socket = self.connect().await?;
            let mut w_socket_guard = self.socket.write().await;

            *w_socket_guard = Some(socket);

            if self.keepwarm || !self.session_ttl.is_zero() {
                let mut session_guard = self.session_expired_at.write().await;
                *session_guard = Some(SystemTime::now().checked_add(self.session_ttl).unwrap());
            }
        }

        let w_socket_guard = self.socket.read().await;
        let socket = w_socket_guard.as_ref().unwrap();

        if let Err(err) = socket.send(payload).await {
            return Err(TunnelError::IO((err.to_string(), err.raw_os_error().unwrap_or(DEFAULT_ERROR_CODE))));
        }

        debug!("{} bytes written to {}", payload.len(), self.addr.unwrap().to_string());

        if self.keepwarm {
            let mut session_guard = self.session_expired_at.write().await;
            *session_guard = Some(SystemTime::now().checked_add(self.session_ttl).unwrap());
        }

        Ok(payload.len())
    }

    async fn recv(&self, buffer: &mut [u8]) -> anyhow::Result<usize, TunnelError> {
        let w_socket_guard = self.socket.read().await;
        let socket = w_socket_guard.as_ref().unwrap();

        match socket.recv_from(buffer).await {
            Ok((n, addr)) => {
                debug!("{} bytes read from {}", n, addr.to_string());
                Ok(n)
            }
            Err(err) => Err(TunnelError::IO((err.to_string(), err.raw_os_error().unwrap_or(DEFAULT_ERROR_CODE)))),
        }
    }

    async fn recv_exact(&self, buffer: &mut [u8]) -> anyhow::Result<usize, TunnelError> {
        self.recv(buffer).await
    }

    async fn check_connect(&self) -> anyhow::Result<(), TunnelError> {
        let _ = self.connect().await?;
        Ok(())
    }
}

impl Default for UDPTunnel {
    fn default() -> Self {
        Self::new()
    }
}

impl UDPTunnel {
    pub fn new() -> Self {
        Self {
            socket: RwLock::new(None),
            addr: None,

            session_expired_at: RwLock::new(None),
            session_ttl: Duration::ZERO,
            keepalive: false,
            keepwarm: false,
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

    async fn connect(&self) -> anyhow::Result<UdpSocket, TunnelError> {
        match UdpSocket::bind("0.0.0.0:0").await {
            Ok(socket) => match socket.connect(self.addr.unwrap()).await {
                Ok(_) => {
                    debug!(
                        "{} socket connected to {}",
                        socket.local_addr().unwrap().to_string(),
                        socket.peer_addr().unwrap().to_string()
                    );

                    Ok(socket)
                }
                Err(err) => Err(TunnelError::Connection((err.to_string(), err.raw_os_error().unwrap_or(CONNECT_ERROR)))),
            },
            Err(err) => Err(TunnelError::Connection((err.to_string(), err.raw_os_error().unwrap_or(CONNECT_ERROR)))),
        }
    }
}

// TODO(nosiee): need to periodically send some small payload to the peer socket to keep the NAT
// cache alive.
