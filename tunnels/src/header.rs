use bytes::{Bytes, BytesMut};
use std::net::SocketAddr;
use std::net::{IpAddr, Ipv4Addr};

pub const HEADER_SIZE: usize = 0x10;
pub const MAX_IP_FRAME_SIZE: usize = 0x400;

#[derive(Debug, Clone)]
pub struct HeaderFrame {
    pub frame_size: u32,
    pub primary_node_ip: Ipv4Addr,
    pub primary_node_port: u16,
}

pub fn extend_payload(payload: &[u8], pnode_addr: Option<SocketAddr>) -> Bytes {
    let extended_size: usize = payload.len() + HEADER_SIZE;
    let mut extended_buffer = BytesMut::zeroed(extended_size);

    extended_buffer[HEADER_SIZE..].copy_from_slice(payload);

    total_packet_size(&mut extended_buffer);

    if let Some(node_addr) = pnode_addr {
        primary_node(&mut extended_buffer, &node_addr);
    }

    extended_buffer.freeze()
}

pub fn decode(buf: [u8; HEADER_SIZE]) -> HeaderFrame {
    HeaderFrame {
        frame_size: u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]),
        primary_node_ip: Ipv4Addr::from(u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]])),
        primary_node_port: u16::from_be_bytes([buf[8], buf[9]]),
    }
}

fn total_packet_size(payload: &mut [u8]) {
    if payload.len() >= MAX_IP_FRAME_SIZE {
        panic!("unexpected: the ip frame size {} >= {}", payload.len(), MAX_IP_FRAME_SIZE)
    }

    let len = u32::try_from(payload.len()).unwrap();
    payload[0..4].copy_from_slice(&len.to_be_bytes());
}

fn primary_node(payload: &mut [u8], node_addr: &SocketAddr) {
    if let IpAddr::V4(ipv4) = node_addr.ip() {
        payload[4..8].copy_from_slice(&ipv4.octets());
        payload[8..10].copy_from_slice(&node_addr.port().to_be_bytes());
    }
}
