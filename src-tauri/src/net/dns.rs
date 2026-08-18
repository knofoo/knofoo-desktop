// Minimal DNS forwarder for guest VMs.
// Intercepts DNS queries on 53/UDP, forwards to host resolver.

use std::net::UdpSocket;

pub struct DnsForwarder {
    upstream: String, // e.g. "8.8.8.8:53"
}

impl DnsForwarder {
    pub fn new(upstream: &str) -> Self {
        Self { upstream: upstream.to_string() }
    }

    pub fn default() -> Self {
        Self::new("8.8.8.8:53")
    }

    // Returns DNS response Ethernet frame, or None on failure
    pub fn handle(&self, frame: &[u8]) -> Option<Vec<u8>> {
        if frame.len() < 42 { return None; }

        let eth_type = u16::from_be_bytes([frame[12], frame[13]]);
        if eth_type != 0x0800 { return None; }

        let ip_proto = frame[23];
        if ip_proto != 17 { return None; }

        let udp_dst = u16::from_be_bytes([frame[36], frame[37]]);
        if udp_dst != 53 { return None; }

        let src_ip  = &frame[26..30];
        let src_mac = &frame[6..12];
        let src_port = u16::from_be_bytes([frame[34], frame[35]]);
        let udp_len  = u16::from_be_bytes([frame[38], frame[39]]) as usize;
        if frame.len() < 42 + udp_len.saturating_sub(8) { return None; }
        let dns_query = &frame[42..42 + udp_len.saturating_sub(8)];

        // Forward to upstream DNS
        let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
        sock.set_read_timeout(Some(std::time::Duration::from_millis(500))).ok()?;
        sock.send_to(dns_query, &self.upstream).ok()?;

        let mut buf = [0u8; 512];
        let (n, _) = sock.recv_from(&mut buf).ok()?;
        let response = &buf[..n];

        Some(self.wrap_udp_response(response, src_mac, src_ip, src_port))
    }

    fn wrap_udp_response(
        &self,
        payload: &[u8],
        dst_mac: &[u8],
        dst_ip: &[u8],
        dst_port: u16,
    ) -> Vec<u8> {
        let server_mac = [0x52u8, 0x54, 0x00, 0xD0, 0x53, 0x01];
        let server_ip  = [10, 0, 0, 1]; // gateway IP

        let udp_len = (8 + payload.len()) as u16;
        let ip_len  = (20 + udp_len) as u16;
        let total   = (14 + ip_len) as usize;
        let mut pkt = vec![0u8; total];

        pkt[0..6].copy_from_slice(&dst_mac[..6.min(dst_mac.len())]);
        pkt[6..12].copy_from_slice(&server_mac);
        pkt[12..14].copy_from_slice(&[0x08, 0x00]);

        pkt[14] = 0x45;
        pkt[16..18].copy_from_slice(&ip_len.to_be_bytes());
        pkt[23] = 17;
        pkt[26..30].copy_from_slice(&server_ip);
        pkt[30..34].copy_from_slice(&dst_ip[..4.min(dst_ip.len())]);

        pkt[34..36].copy_from_slice(&53u16.to_be_bytes());
        pkt[36..38].copy_from_slice(&dst_port.to_be_bytes());
        pkt[38..40].copy_from_slice(&udp_len.to_be_bytes());

        let off = 42;
        pkt[off..off + payload.len()].copy_from_slice(payload);

        pkt
    }
}
