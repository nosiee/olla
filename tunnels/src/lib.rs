pub mod errors;
pub mod header;
pub mod incoming;
pub mod outgoing;

use std::sync::Arc;
use tokio::sync::broadcast::Sender;

use errors::TunnelError;
use types::*;

#[derive(Debug, Clone)]
pub enum TunnelType {
    Tls,
    Rtmp,
    Udp,
    Tcp,
    Unknown,
}

impl From<String> for TunnelType {
    fn from(t: String) -> Self {
        match t.as_str() {
            "tls" => Self::Tls,
            "rtmp" => Self::Rtmp,
            "udp" => Self::Udp,
            "tcp" => Self::Tcp,
            _ => Self::Unknown,
        }
    }
}

pub trait AsyncOutgoingTunnel {
    fn send(&self, payload: &[u8]) -> impl std::future::Future<Output = Result<usize, TunnelError>> + Send;
    fn recv(&self, buffer: &mut [u8]) -> impl std::future::Future<Output = Result<usize, TunnelError>> + Send;
    fn recv_exact(&self, buffer: &mut [u8]) -> impl std::future::Future<Output = Result<usize, TunnelError>> + Send;
    fn check_connect(&self) -> impl std::future::Future<Output = Result<(), TunnelError>> + Send;
}

pub trait AsyncIncomingTunnel {
    fn forward(self: Arc<Self>, tx: Sender<PacketCoordinatorMessage>) -> impl std::future::Future<Output = Result<(), TunnelError>> + Send;
    fn write(&self, peer: String, payload: &[u8]) -> impl ::std::future::Future<Output = Result<usize, TunnelError>> + Send;
}

pub trait SyncOutgoingTunnel {
    fn send(&self, payload: &[u8]) -> Result<usize, TunnelError>;
    fn recv(&self, buffer: &mut [u8]) -> Result<usize, TunnelError>;
    fn recv_exact(&self, buffer: &mut [u8]) -> Result<usize, TunnelError>;
    fn check_connect() -> Result<(), TunnelError>;
}

pub trait SyncIncomingTunnel {
    fn forward(self: Arc<Self>, tx: Sender<PacketCoordinatorMessage>) -> Result<(), TunnelError>;
    fn write(&self, peer: String, payload: &[u8]) -> Result<usize, TunnelError>;
}
