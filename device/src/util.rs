use super::packet::*;
use pnet::packet::{ethernet::EtherTypes, ip::IpNextHeaderProtocols, Packet};

pub fn get_source_identity(buf: &[u8]) -> Option<String> {
    let eth_pkt = from_ip_payload(buf)?;

    let identity = match eth_pkt.get_ethertype() {
        EtherTypes::Ipv4 => {
            let ip_pkt = to_ipv4(eth_pkt.payload())?;
            let src_ip = ip_pkt.get_source();

            let src_id = match ip_pkt.get_next_level_protocol() {
                IpNextHeaderProtocols::Tcp => {
                    let tcp_pkt = to_tcp(ip_pkt.payload())?;
                    tcp_pkt.get_source()
                }
                IpNextHeaderProtocols::Udp => {
                    let udp_pkt = to_udp(ip_pkt.payload())?;
                    udp_pkt.get_source()
                }
                IpNextHeaderProtocols::Icmp => {
                    return None;
                }
                _ => return None,
            };

            format!("{}:{}", src_ip, src_id)
        }
        EtherTypes::Ipv6 => {
            let ip_pkt = to_ipv6(eth_pkt.payload())?;
            let src_ip = ip_pkt.get_source();

            let src_id = match ip_pkt.get_next_header() {
                IpNextHeaderProtocols::Tcp => {
                    let tcp_pkt = to_tcp(ip_pkt.payload())?;
                    tcp_pkt.get_source()
                }
                IpNextHeaderProtocols::Udp => {
                    let udp_pkt = to_udp(ip_pkt.payload())?;
                    udp_pkt.get_source()
                }
                IpNextHeaderProtocols::Icmpv6 => {
                    return None;
                }
                _ => return None,
            };

            format!("{}:{}", src_ip, src_id)
        }
        _ => return None,
    };

    Some(identity)
}

pub fn get_destination_identity(buf: &[u8]) -> Option<String> {
    let eth_pkt = from_ip_payload(buf)?;

    let identity = match eth_pkt.get_ethertype() {
        EtherTypes::Ipv4 => {
            let ip_pkt = to_ipv4(eth_pkt.payload())?;
            let dst_ip = ip_pkt.get_destination();

            let dst_id = match ip_pkt.get_next_level_protocol() {
                IpNextHeaderProtocols::Tcp => {
                    let tcp_pkt = to_tcp(ip_pkt.payload())?;
                    tcp_pkt.get_destination()
                }
                IpNextHeaderProtocols::Udp => {
                    let udp_pkt = to_udp(ip_pkt.payload())?;
                    udp_pkt.get_destination()
                }
                IpNextHeaderProtocols::Icmp => {
                    return None;
                }
                _ => return None,
            };

            format!("{}:{}", dst_ip, dst_id)
        }
        EtherTypes::Ipv6 => {
            let ip_pkt = to_ipv6(eth_pkt.payload())?;
            let dst_ip = ip_pkt.get_destination();

            let dst_id = match ip_pkt.get_next_header() {
                IpNextHeaderProtocols::Tcp => {
                    let tcp_pkt = to_tcp(ip_pkt.payload())?;
                    tcp_pkt.get_destination()
                }
                IpNextHeaderProtocols::Udp => {
                    let udp_pkt = to_udp(ip_pkt.payload())?;
                    udp_pkt.get_destination()
                }
                IpNextHeaderProtocols::Icmpv6 => {
                    return None;
                }
                _ => return None,
            };

            format!("{}:{}", dst_ip, dst_id)
        }
        _ => return None,
    };

    Some(identity)
}
