use super::error::TunnelError;

#[derive(Debug, Clone)]
pub enum TunnelType {
    TLS,
    RTMP,
    Unknown,
}

impl From<String> for TunnelType {
    fn from(t: String) -> Self {
        match t.as_str() {
            "tls" => Self::TLS,
            "rtmp" => Self::RTMP,

            _ => Self::Unknown,
        }
    }
}

pub trait AsyncTunnel {
    fn send(
        &self,
        payload: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<usize, TunnelError>> + Send;

    fn recv(
        &self,
        buffer: &mut [u8],
    ) -> impl std::future::Future<Output = Result<usize, TunnelError>> + Send;

    fn check_connect(&self) -> impl std::future::Future<Output = Result<(), TunnelError>> + Send;
}

pub trait SyncTunnel {
    fn send(&self, payload: &[u8]) -> Result<usize, TunnelError>;
    fn recv(&self, buffer: &mut [u8]) -> Result<usize, TunnelError>;
    fn check_connect() -> Result<(), TunnelError>;
}
