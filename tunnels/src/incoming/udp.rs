use bytes::BytesMut;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::{net::UdpSocket, sync::RwLock};
use tracing::{debug, error};

use crate::errors::NO_IDENTITY_FOUND;
use crate::{AsyncIncomingTunnel, errors::DEFAULT_ERROR_CODE, errors::TunnelError};
use device::Message;

pub struct UDPTunnel {
    socket: Arc<UdpSocket>,
    peers: RwLock<HashMap<String, String>>,
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

                match device::util::get_source_identity(&buffer) {
                    Some(identity) => {
                        let mut peers_guard = self.peers.write().await;
                        peers_guard.insert(identity, addr.to_string());

                        if let Err(err) = tx.send(buffer.freeze()).await {
                            error!("failed to send incoming udp payload to the tx: {}", err);
                        }
                    }
                    None => debug!("{} packet omitted, no source identity found", hex::encode(&buffer)),
                }
            }
        });

        Ok(())
    }

    async fn write(&self, peer: String, payload: &[u8]) -> anyhow::Result<usize, TunnelError> {
        let peers_guard = self.peers.read().await;

        if let Some(peer_addr) = peers_guard.get(&peer) {
            let peer_addr: SocketAddr = peer_addr.parse().unwrap();

            return match self.socket.send_to(payload, peer_addr).await {
                Ok(n) => {
                    debug!("{} bytes written to {}", payload.len(), peer_addr);
                    Ok(n)
                }
                Err(err) => Err(TunnelError::IO((err.to_string(), err.raw_os_error().unwrap_or(DEFAULT_ERROR_CODE)))),
            };
        }

        Err(TunnelError::Connection((
            "failed to write payload, no identity found".into(),
            NO_IDENTITY_FOUND,
        )))
    }
}

impl UDPTunnel {
    pub async fn new(addr: SocketAddr) -> Self {
        Self {
            socket: Arc::new(UdpSocket::bind(addr).await.unwrap()),
            peers: RwLock::new(HashMap::new()),
        }
    }
}
