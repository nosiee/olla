use anyhow::anyhow;

pub type ErrorCode = i32;
pub type ErrorMessage = (String, ErrorCode);

// NOTE(nosiee): keep userspace error codes above zero to avoid conflicts with OS codes
pub const DEFAULT_ERROR_CODE: ErrorCode = -1;
pub const CONNECT_ERROR: ErrorCode = -2;
pub const PAYLOAD_SIZE_OVERFLOW: ErrorCode = -3;
pub const SNI_PARSING_ERROR: ErrorCode = -4;
pub const TLS_CONNECT_ERROR: ErrorCode = -5;
pub const NO_PEER_FOUND: ErrorCode = -6;

#[derive(Debug)]
pub enum TunnelError {
    IO(ErrorMessage),
    Connection(ErrorMessage),
    Strict(ErrorMessage),
}

impl From<TunnelError> for anyhow::Error {
    fn from(e: TunnelError) -> Self {
        let error_text = match e {
            TunnelError::IO(e) => format!("io error: {}, code: {}", e.0, e.1),
            TunnelError::Connection(e) => format!("connection error: {}, code: {}", e.0, e.1),
            TunnelError::Strict(e) => format!("pedantic error: {}, code: {}", e.0, e.1),
        };

        anyhow!(error_text)
    }
}
