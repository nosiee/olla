use bytes::BytesMut;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error};

use crate::{AsyncIncomingTunnel, errors::DEFAULT_ERROR_CODE, errors::TunnelError};
use device::Message;

pub struct UDPTunnel {
    socket: Arc<UdpSocket>,
}

impl AsyncIncomingTunnel for UDPTunnel {
    async fn forward(self: Arc<Self>, tx: Sender<Message>) -> anyhow::Result<(), TunnelError> {
        tokio::spawn(async move {
            loop {
                // FIXME(nosiee): should use device MTU
                let mut buffer = BytesMut::zeroed(1500);

                let r = self.socket.recv_from(&mut buffer).await;
                if r.is_err() {
                    error!("failed to read incoming udp payload: {}", r.err().unwrap());
                    continue;
                }

                let (n, addr) = r.unwrap();
                buffer.truncate(n);
                debug!("{} bytes read from {}", n, addr.to_string());

                if let Err(err) = tx.send(buffer.freeze()).await {
                    error!("failed to send incoming udp payload to the tx: {}", err);
                }
            }
        });

        Ok(())
    }

    async fn write(&self, peer: String, payload: &[u8]) -> anyhow::Result<usize, TunnelError> {
        let peer_addr: SocketAddr = peer.parse().unwrap();

        match self.socket.send_to(payload, peer_addr).await {
            Ok(n) => {
                debug!("{} bytes written to {}", payload.len(), peer_addr);
                Ok(n)
            }
            Err(err) => Err(TunnelError::IO((err.to_string(), err.raw_os_error().unwrap_or(DEFAULT_ERROR_CODE)))),
        }
    }
}

impl UDPTunnel {
    pub async fn new(addr: SocketAddr) -> Self {
        Self {
            socket: Arc::new(UdpSocket::bind(addr).await.unwrap()),
        }
    }
}

// TODO(nosiee): need to periodically send some small payload to the peer socket to keep the NAT
// cache alive.
