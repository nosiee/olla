pub fn correct(packet: &mut [u8]) -> anyhow::Result<(), &'static str> {
    if packet.len() < 20 {
        return Err("packet too short for IPv4");
    }

    let ihl = (packet[0] & 0x0f) as usize;
    let ip_header_len = ihl.checked_mul(4).ok_or("invalid IHL")?;
    if ip_header_len < 20 || packet.len() < ip_header_len {
        return Err("invalid IP header length");
    }

    let total_len = u16::from_be_bytes([packet[2], packet[3]]) as usize;
    if total_len < ip_header_len || packet.len() < total_len {
        return Err("invalid total length");
    }

    let proto = packet[9];
    let tcp_offset = ip_header_len;
    let tcp_len = total_len - ip_header_len;
    if tcp_len < 20 {
        return Err("TCP segment too short");
    }
    if tcp_offset + 16 + 1 >= packet.len() {
        return Err("packet too short for TCP checksum field");
    }

    fn sum_words(mut data: &[u8]) -> u32 {
        let mut s: u32 = 0;
        while data.len() >= 2 {
            s += u16::from_be_bytes([data[0], data[1]]) as u32;
            data = &data[2..];
        }
        if data.len() == 1 {
            s += (u16::from(data[0]) << 8) as u32;
        }
        s
    }

    let mut sum: u32 = 0;
    sum += sum_words(&packet[12..16]);
    sum += sum_words(&packet[16..20]);
    sum += proto as u32;
    sum += (tcp_len as u32) & 0xffff;

    let tcp_segment = &packet[tcp_offset..tcp_offset + tcp_len];
    if tcp_len >= 18 {
        sum += sum_words(&tcp_segment[..16]);
        sum += sum_words(&tcp_segment[18..]);
    } else {
        return Err("tcp_len too small");
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    let csum = !(sum as u16);
    let off = tcp_offset + 16;
    packet[off] = (csum >> 8) as u8;
    packet[off + 1] = (csum & 0xff) as u8;

    Ok(())
}
