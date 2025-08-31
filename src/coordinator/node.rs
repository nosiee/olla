use crate::tunnels::tunnel::{AsyncTunnel, TunnelType};

use std::net::SocketAddr;

#[derive(Debug)]
pub struct Node<T: AsyncTunnel> {
    pub id: String,
    pub addr: SocketAddr,
    pub tunnel_type: TunnelType,
    pub tunnel: T,
}
