use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tracing::debug;

use super::errors::*;
use super::header;

#[derive(Debug)]
pub struct OutgoingTunnel {
    socket: RwLock<Option<UdpSocket>>,
    addr: Option<SocketAddr>,

    keepalive: u64,
    pnode_addr: Option<SocketAddr>,
}

impl Default for OutgoingTunnel {
    fn default() -> Self {
        Self::new()
    }
}

impl OutgoingTunnel {
    pub fn new() -> Self {
        Self {
            socket: RwLock::new(None),
            addr: None,

            keepalive: 0,
            pnode_addr: None,
        }
    }

    pub fn set_addr(mut self, addr: SocketAddr) -> Self {
        self.addr = Some(addr);
        self
    }

    pub fn set_keepalive(mut self, keepalive: u64) -> Self {
        self.keepalive = keepalive;
        self
    }

    pub fn set_primary_node(mut self, pnode_addr: SocketAddr) -> Self {
        self.pnode_addr = Some(pnode_addr);
        self
    }

    pub async fn send(&self, payload: &[u8]) -> anyhow::Result<usize, TunnelError> {
        if self.socket.read().await.is_none() {
            let socket = self.connect().await?;
            let mut w_socket_guard = self.socket.write().await;

            *w_socket_guard = Some(socket);
        }

        let w_socket_guard = self.socket.read().await;
        let socket = w_socket_guard.as_ref().unwrap();
        let payload = header::extend_payload(payload, self.pnode_addr);

        if let Err(err) = socket.send(&payload).await {
            return Err(TunnelError::IO((err.to_string(), err.raw_os_error().unwrap_or(DEFAULT_ERROR_CODE))));
        }

        debug!("{} bytes written to {}", payload.len(), self.addr.unwrap().to_string());
        Ok(payload.len())
    }

    pub async fn recv(&self, buffer: &mut [u8]) -> anyhow::Result<usize, TunnelError> {
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

    pub async fn recv_exact(&self, buffer: &mut [u8]) -> anyhow::Result<usize, TunnelError> {
        self.recv(buffer).await
    }

    pub async fn check_connect(&self) -> anyhow::Result<(), TunnelError> {
        let _ = self.connect().await?;
        Ok(())
    }

    async fn connect(&self) -> anyhow::Result<UdpSocket, TunnelError> {
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(socket) => socket,
            Err(err) => return Err(TunnelError::Connection((err.to_string(), err.raw_os_error().unwrap_or(CONNECT_ERROR)))),
        };

        if let Err(err) = socket.connect(self.addr.unwrap()).await {
            return Err(TunnelError::Connection((err.to_string(), err.raw_os_error().unwrap_or(CONNECT_ERROR))));
        }

        debug!(
            "{} socket connected to {}",
            socket.local_addr().unwrap().to_string(),
            socket.peer_addr().unwrap().to_string()
        );

        Ok(socket)
    }
}
