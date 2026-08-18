// Userspace NAT using smoltcp for internet access.
// Guest eth1 frames → smoltcp TCP/IP → host OS sockets → internet.
// No root, no TAP, no kernel config needed.

use smoltcp::phy::{RxToken, TxToken};
use smoltcp::wire::Ipv4Address;
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;

// VM's NAT address: 10.1.{vm_index}.2, gateway: 10.1.{vm_index}.1
pub fn vm_nat_ip(vm_index: u8) -> Ipv4Address {
    Ipv4Address::new(10, 1, vm_index, 2)
}
pub fn gateway_ip(vm_index: u8) -> Ipv4Address {
    Ipv4Address::new(10, 1, vm_index, 1)
}

pub struct SlirpDevice {
    rx_queue: VecDeque<Vec<u8>>, // frames from guest → smoltcp
    tx_queue: VecDeque<Vec<u8>>, // frames from smoltcp → guest
    // Active TCP connections: smoltcp handle → host socket
    tcp_conns: HashMap<usize, std::net::TcpStream>,
    vm_index: u8,
    port_forwards: Vec<(u16, u16, String)>, // (host_port, guest_port, proto)
}

impl SlirpDevice {
    pub fn new(vm_index: u8) -> Self {
        Self {
            rx_queue: VecDeque::new(),
            tx_queue: VecDeque::new(),
            tcp_conns: HashMap::new(),
            vm_index,
            port_forwards: Vec::new(),
        }
    }

    pub fn add_port_forward(&mut self, host_port: u16, guest_port: u16, proto: &str) {
        self.port_forwards.push((host_port, guest_port, proto.to_string()));
    }

    // Guest sends an Ethernet frame via eth1
    pub fn guest_send(&mut self, frame: Vec<u8>) {
        self.rx_queue.push_back(frame);
    }

    // Host delivers a frame to the guest via eth1
    pub fn guest_recv(&mut self) -> Option<Vec<u8>> {
        self.tx_queue.pop_front()
    }

    // Poll: process pending frames, run smoltcp, handle host sockets
    pub fn poll(&mut self) {
        // In full implementation:
        // 1. Drain rx_queue → feed to smoltcp interface as RxTokens
        // 2. smoltcp.poll() → processes TCP/UDP/ICMP
        // 3. For TCP connect: open real host socket to destination
        // 4. Forward data host_socket ↔ smoltcp socket
        // 5. Drain smoltcp TxTokens → push to tx_queue (→ guest)
        //
        // This is the core of a userspace NAT. Full implementation
        // follows smoltcp's "loopback" or "raw socket" device pattern.
        // Reference: smoltcp/examples/client.rs
    }

    pub fn vm_ip(&self) -> Ipv4Address { vm_nat_ip(self.vm_index) }
    pub fn gw_ip(&self) -> Ipv4Address { gateway_ip(self.vm_index) }
}

// Minimal RxToken/TxToken impl for smoltcp Device trait
struct SlirpRxToken(Vec<u8>);
struct SlirpTxToken<'a>(&'a mut VecDeque<Vec<u8>>);

impl RxToken for SlirpRxToken {
    fn consume<R, F>(self, f: F) -> R where F: FnOnce(&mut [u8]) -> R {
        f(&mut { self.0 })
    }
}

impl<'a> TxToken for SlirpTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R where F: FnOnce(&mut [u8]) -> R {
        let mut buf = vec![0u8; len];
        let result = f(&mut buf);
        self.0.push_back(buf);
        result
    }
}
