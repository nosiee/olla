use bytes::BytesMut;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::broadcast::Sender;
use tracing::error;
use types::*;

use crate::{AsyncIncomingTunnel, errors::*};

pub struct UDPTunnel {
    id: String,
    socket: Arc<UdpSocket>,
}

impl AsyncIncomingTunnel for UDPTunnel {
    async fn forward(self: Arc<Self>, tx: Sender<PacketCoordinatorMessage>) -> anyhow::Result<(), TunnelError> {
        tokio::spawn(async move {
            loop {
                // FIXME(nosiee): should use device MTU + HEADER_SIZE
                let mut buffer = BytesMut::zeroed(1500);

                let r = self.socket.recv_from(&mut buffer).await;
                if r.is_err() {
                    error!("failed to read incoming udp payload: {}", r.err().unwrap());
                    continue;
                }

                let (n, addr) = r.unwrap();
                buffer.truncate(n);

                if let Err(err) = tx.send((self.id.clone(), addr.to_string(), buffer.freeze())) {
                    panic!("{}", err);
                }
            }
        });

        Ok(())
    }

    async fn write(&self, peer: String, payload: &[u8]) -> anyhow::Result<usize, TunnelError> {
        let peer: SocketAddr = peer.parse().unwrap();

        match self.socket.send_to(payload, peer).await {
            Ok(n) => Ok(n),
            Err(err) => Err(TunnelError::IO((err.to_string(), err.raw_os_error().unwrap_or(DEFAULT_ERROR_CODE)))),
        }
    }
}

impl UDPTunnel {
    pub async fn new(addr: SocketAddr) -> Self {
        Self {
            id: String::new(),
            socket: Arc::new(UdpSocket::bind(addr).await.unwrap()),
        }
    }
}
