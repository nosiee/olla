use std::net::SocketAddr;
use tunnels::{AsyncOutgoingTunnel, TunnelType};

#[derive(Debug)]
pub struct Node<T: AsyncOutgoingTunnel> {
    pub id: String,
    pub addr: SocketAddr,
    pub tunnel_type: TunnelType,
    pub tunnel: T,
    pub max_fragment_size: usize,
}
