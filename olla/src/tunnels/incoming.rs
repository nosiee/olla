use async_channel::Sender;
use bytes::BytesMut;
use nix::sys::socket::setsockopt;
use nix::sys::socket::sockopt::Ipv4PacketInfo;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::error;

use super::errors::*;
use crate::coordinator::packet::PacketCoordinatorMessage;

pub struct IncomingTunnel {
    addr: SockAddr,
    socket: Option<Arc<UdpSocket>>,
}

impl IncomingTunnel {
    pub fn new(addr: SockAddr) -> Self {
        Self { addr, socket: None }
    }

    pub async fn forward(&mut self, tx: Sender<PacketCoordinatorMessage>) -> anyhow::Result<(), TunnelError> {
        let cpu_cores = match std::thread::available_parallelism() {
            Ok(n) => n.get(),
            Err(err) => {
                error!("failed to get avaialble parallelism count: {}. fallback to default 1", err);
                1_usize
            }
        };

        for _ in 0..cpu_cores {
            let sock = self.recreate_socket(&self.addr).unwrap();
            let tx = tx.clone();

            if self.socket.is_none() {
                self.socket = Some(sock.clone());
            }

            tokio::spawn(async move {
                loop {
                    // FIXME(nosiee): should use device mtu + HEADER_SIZE
                    let mut buffer = BytesMut::zeroed(1500);

                    let r = sock.recv_from(&mut buffer).await;
                    if r.is_err() {
                        error!("failed to read incoming udp payload: {}", r.err().unwrap());
                        continue;
                    }

                    let (n, addr) = r.unwrap();
                    buffer.truncate(n);

                    if let Err(err) = tx.send((addr.to_string(), buffer.freeze())).await {
                        panic!("{}", err);
                    }
                }
            });
        }

        Ok(())
    }

    pub async fn write(&self, peer: String, payload: &[u8]) -> anyhow::Result<usize, TunnelError> {
        let peer: SocketAddr = peer.parse().unwrap();

        match self.socket.as_ref().unwrap().send_to(payload, peer).await {
            Ok(n) => Ok(n),
            Err(err) => Err(TunnelError::IO((err.to_string(), err.raw_os_error().unwrap_or(DEFAULT_ERROR_CODE)))),
        }
    }

    fn recreate_socket(&self, addr: &SockAddr) -> anyhow::Result<Arc<UdpSocket>> {
        let rawfd = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;

        rawfd.set_reuse_port(true)?;
        rawfd.set_cloexec(true)?;
        rawfd.set_nonblocking(true)?;
        rawfd.bind(addr)?;

        setsockopt(&rawfd, Ipv4PacketInfo, &true)?;

        Ok(Arc::new(UdpSocket::from_std(rawfd.into())?))
    }
}
