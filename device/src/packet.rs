use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::icmp::echo_reply::EchoReplyPacket;
use pnet::packet::icmp::echo_request::EchoRequestPacket;
use pnet::packet::icmp::IcmpPacket;
use pnet::packet::icmpv6::Icmpv6Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::{ipv4::Ipv4Packet, ipv6::Ipv6Packet};
use pnet::util::MacAddr;

pub const MAX_ETH_FRAME_SIZE: usize = 18;

pub fn from_ip_payload(buf: &[u8]) -> Option<EthernetPacket> {
    let eth_buf = vec![0; buf.len() + MAX_ETH_FRAME_SIZE];
    let mut fake_eth_frame = MutableEthernetPacket::owned(eth_buf).unwrap();
    let ip_packet = Ipv4Packet::new(buf).unwrap();

    fake_eth_frame.set_destination(MacAddr(0, 0, 0, 0, 0, 0));
    fake_eth_frame.set_source(MacAddr(0, 0, 0, 0, 0, 0));
    fake_eth_frame.set_payload(buf);

    match ip_packet.get_version() {
        4 => fake_eth_frame.set_ethertype(EtherTypes::Ipv4),
        6 => fake_eth_frame.set_ethertype(EtherTypes::Ipv6),
        _ => return None,
    }

    Some(fake_eth_frame.consume_to_immutable())
}

pub fn to_ipv4<'a>(packet: &'a [u8]) -> Option<Ipv4Packet<'a>> {
    Ipv4Packet::new(packet)
}

pub fn to_ipv6<'a>(packet: &'a [u8]) -> Option<Ipv6Packet<'a>> {
    Ipv6Packet::new(packet)
}

pub fn to_tcp<'a>(packet: &'a [u8]) -> Option<TcpPacket<'a>> {
    TcpPacket::new(packet)
}

pub fn to_udp<'a>(packet: &'a [u8]) -> Option<UdpPacket<'a>> {
    UdpPacket::new(packet)
}

pub fn to_icmp<'a>(packet: &'a [u8]) -> Option<IcmpPacket<'a>> {
    IcmpPacket::new(packet)
}

pub fn to_icmpv6<'a>(packet: &'a [u8]) -> Option<Icmpv6Packet<'a>> {
    Icmpv6Packet::new(packet)
}

pub fn to_icmp_request<'a>(packet: &'a [u8]) -> Option<EchoRequestPacket<'a>> {
    EchoRequestPacket::new(packet)
}

pub fn to_icmp_reply<'a>(packet: &'a [u8]) -> Option<EchoReplyPacket<'a>> {
    EchoReplyPacket::new(packet)
}
