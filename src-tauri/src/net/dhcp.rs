// Tiny DHCP server for LAN (eth0) and NAT (eth1) interfaces.
// Responds to DHCPDISCOVER and DHCPREQUEST with deterministic IPs.

// LAN:  10.0.{graph_hash}.{vm_index}/24, GW: 10.0.{graph_hash}.1
// NAT:  10.1.{vm_index}.2/24,            GW: 10.1.{vm_index}.1

pub struct DhcpServer {
    server_mac: [u8; 6],
    server_ip: [u8; 4],
    client_ip: [u8; 4],
    gateway_ip: [u8; 4],
    subnet_mask: [u8; 4],
    dns_ip: [u8; 4],
}

impl DhcpServer {
    pub fn for_lan(graph_index: u8, vm_index: u8, server_mac: [u8; 6]) -> Self {
        Self {
            server_mac,
            server_ip:   [10, 0, graph_index, 1],
            client_ip:   [10, 0, graph_index, vm_index + 2],
            gateway_ip:  [10, 0, graph_index, 1],
            subnet_mask: [255, 255, 255, 0],
            dns_ip:      [10, 0, graph_index, 1],
        }
    }

    pub fn for_nat(vm_index: u8, server_mac: [u8; 6]) -> Self {
        Self {
            server_mac,
            server_ip:   [10, 1, vm_index, 1],
            client_ip:   [10, 1, vm_index, 2],
            gateway_ip:  [10, 1, vm_index, 1],
            subnet_mask: [255, 255, 255, 0],
            dns_ip:      [8, 8, 8, 8],
        }
    }

    // Returns a DHCP offer/ack UDP payload for a given request frame.
    pub fn handle(&self, frame: &[u8]) -> Option<Vec<u8>> {
        // Parse outer Ethernet + IP + UDP headers to find DHCP payload
        if frame.len() < 42 { return None; }
        let eth_type = u16::from_be_bytes([frame[12], frame[13]]);
        if eth_type != 0x0800 { return None; } // IPv4 only

        let ip_proto = frame[23];
        if ip_proto != 17 { return None; } // UDP only

        let udp_dst = u16::from_be_bytes([frame[36], frame[37]]);
        if udp_dst != 67 { return None; } // DHCP server port

        let dhcp = &frame[42..];
        if dhcp.len() < 240 { return None; }

        let op    = dhcp[0];
        let htype = dhcp[1];
        let hlen  = dhcp[2] as usize;
        let xid   = &dhcp[4..8];
        let chaddr = &dhcp[28..28 + hlen.min(6)];

        if op != 1 { return None; } // BOOTREQUEST only

        let msg_type = self.dhcp_msg_type(dhcp)?;

        let (reply_type, is_offer) = match msg_type {
            1 => (2u8, true),  // DISCOVER → OFFER
            3 => (5u8, false), // REQUEST  → ACK
            _ => return None,
        };

        Some(self.build_reply(xid, chaddr, reply_type))
    }

    fn dhcp_msg_type(&self, dhcp: &[u8]) -> Option<u8> {
        // Parse options starting at byte 240
        let mut i = 240;
        while i < dhcp.len() {
            let opt = dhcp[i];
            if opt == 255 { break; } // END
            if opt == 0 { i += 1; continue; } // PAD
            if i + 1 >= dhcp.len() { break; }
            let len = dhcp[i + 1] as usize;
            if opt == 53 && len == 1 && i + 2 < dhcp.len() {
                return Some(dhcp[i + 2]);
            }
            i += 2 + len;
        }
        None
    }

    fn build_reply(&self, xid: &[u8], chaddr: &[u8], msg_type: u8) -> Vec<u8> {
        let mut dhcp = vec![0u8; 300];
        dhcp[0] = 2; // BOOTREPLY
        dhcp[1] = 1; // HTYPE ethernet
        dhcp[2] = 6; // HLEN
        dhcp[3] = 0; // HOPS
        dhcp[4..8].copy_from_slice(xid);
        dhcp[16..20].copy_from_slice(&self.client_ip);   // yiaddr
        dhcp[20..24].copy_from_slice(&self.server_ip);   // siaddr
        dhcp[28..34].copy_from_slice(&chaddr[..6.min(chaddr.len())]);
        // Magic cookie
        dhcp[236] = 99; dhcp[237] = 130; dhcp[238] = 83; dhcp[239] = 99;
        // Options
        let opts = &mut dhcp[240..];
        let mut i = 0;
        // Message type
        opts[i] = 53; opts[i+1] = 1; opts[i+2] = msg_type; i += 3;
        // Server identifier
        opts[i] = 54; opts[i+1] = 4;
        opts[i+2..i+6].copy_from_slice(&self.server_ip); i += 6;
        // Subnet mask
        opts[i] = 1; opts[i+1] = 4;
        opts[i+2..i+6].copy_from_slice(&self.subnet_mask); i += 6;
        // Router
        opts[i] = 3; opts[i+1] = 4;
        opts[i+2..i+6].copy_from_slice(&self.gateway_ip); i += 6;
        // DNS
        opts[i] = 6; opts[i+1] = 4;
        opts[i+2..i+6].copy_from_slice(&self.dns_ip); i += 6;
        // Lease time: 24h
        opts[i] = 51; opts[i+1] = 4;
        let lease = 86400u32.to_be_bytes();
        opts[i+2..i+6].copy_from_slice(&lease); i += 6;
        // END
        opts[i] = 255;

        self.wrap_udp(dhcp)
    }

    fn wrap_udp(&self, dhcp_payload: Vec<u8>) -> Vec<u8> {
        let udp_len = (8 + dhcp_payload.len()) as u16;
        let ip_len  = (20 + udp_len) as u16;
        let total   = (14 + ip_len) as usize;
        let mut pkt = vec![0u8; total];

        // Ethernet header: dst=broadcast, src=server
        pkt[0..6].copy_from_slice(&[0xFF; 6]);
        pkt[6..12].copy_from_slice(&self.server_mac);
        pkt[12..14].copy_from_slice(&[0x08, 0x00]); // IPv4

        // IP header
        pkt[14] = 0x45; // version + IHL
        pkt[16..18].copy_from_slice(&ip_len.to_be_bytes());
        pkt[23] = 17; // UDP
        pkt[26..30].copy_from_slice(&self.server_ip);
        pkt[30..34].copy_from_slice(&[255; 4]); // dst broadcast

        // UDP
        pkt[34..36].copy_from_slice(&67u16.to_be_bytes()); // src port
        pkt[36..38].copy_from_slice(&68u16.to_be_bytes()); // dst port
        pkt[38..40].copy_from_slice(&udp_len.to_be_bytes());

        // DHCP payload
        let off = 42;
        pkt[off..off + dhcp_payload.len()].copy_from_slice(&dhcp_payload);

        // Compute IP checksum
        let checksum = ip_checksum(&pkt[14..34]);
        pkt[24..26].copy_from_slice(&checksum.to_be_bytes());

        pkt
    }
}

fn ip_checksum(header: &[u8]) -> u16 {
    let mut sum = 0u32;
    for i in (0..header.len()).step_by(2) {
        let word = if i + 1 < header.len() {
            u16::from_be_bytes([header[i], header[i+1]]) as u32
        } else {
            (header[i] as u32) << 8
        };
        sum += word;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}
