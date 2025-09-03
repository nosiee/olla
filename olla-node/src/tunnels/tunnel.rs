use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::error::TunnelError;
use crate::types::Message;

#[derive(Debug, Clone)]
pub enum TunnelType {
    Tls,
    Rtmp,
    Unknown,
}

impl From<String> for TunnelType {
    fn from(t: String) -> Self {
        match t.as_str() {
            "tls" => Self::Tls,
            "rtmp" => Self::Rtmp,

            _ => Self::Unknown,
        }
    }
}

pub trait AsyncTunnel {
    fn forward(self: Arc<Self>, tx: Sender<Message>) -> impl std::future::Future<Output = Result<(), TunnelError>> + Send;
    fn write(&self, peer: String, payload: Vec<u8>) -> impl ::std::future::Future<Output = Result<usize, TunnelError>> + Send;
}
