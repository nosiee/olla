pub const HEADER_SIZE: usize = 0x10;
pub const MAX_IP_FRAME_SIZE: usize = 0x400;

#[derive(Debug, Clone)]
pub struct HeaderFrame {
    pub frame_size: u32,
}

pub fn add(payload: &mut Vec<u8>) {
    resize(payload, HEADER_SIZE);

    total_packet_size(payload);
}

pub fn decode(buf: [u8; HEADER_SIZE]) -> HeaderFrame {
    HeaderFrame {
        frame_size: u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]),
    }
}

fn resize(payload: &mut Vec<u8>, size: usize) {
    let payload_len = payload.len();

    payload.resize(payload_len + size, 0);
    payload.copy_within(0..payload_len, size);
    payload[0..size].fill(0);
}

fn total_packet_size(payload: &mut [u8]) {
    if payload.len() >= MAX_IP_FRAME_SIZE {
        panic!("unexpected: the ip frame size {} >= {}", payload.len(), MAX_IP_FRAME_SIZE)
    }

    let len = u32::try_from(payload.len()).unwrap();
    payload[0..4].copy_from_slice(&len.to_be_bytes());
}
