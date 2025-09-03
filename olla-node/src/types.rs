pub type Message = Vec<u8>;

pub const KB: usize = 1024;
pub const MAX_IP_FRAME_SIZE: usize = 64 * KB;
pub const PACKETS_BUFFER_SIZE: usize = 1024;
pub const HEADER_SIZE: usize = 16;
