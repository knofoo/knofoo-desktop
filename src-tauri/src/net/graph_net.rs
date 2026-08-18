// graph_net: wires switch + slirp + dhcp + dns together per VM.
// Each VM gets a VmNetwork with two virtual NICs.

use std::sync::{Arc, Mutex};
use super::switch::{VirtualSwitch, FrameReceiver};
use super::slirp::SlirpDevice;
use super::dhcp::DhcpServer;
use super::dns::DnsForwarder;

pub struct VmNetwork {
    pub vm_id: String,
    pub vm_index: u8,

    // LAN (eth0)
    pub lan_mac: [u8; 6],
    pub lan_rx: Option<FrameReceiver>,       // frames arriving from switch
    pub lan_pending_tx: Vec<Vec<u8>>,         // frames to inject to guest

    // NAT (eth1)
    pub nat_mac: [u8; 6],
    pub slirp: SlirpDevice,

    // Services
    pub dhcp_lan: DhcpServer,
    pub dhcp_nat: DhcpServer,
    pub dns: DnsForwarder,

    switch: Option<Arc<Mutex<VirtualSwitch>>>,
}

impl VmNetwork {
    pub fn new(
        vm_id: &str,
        vm_index: u8,
        lan_enabled: bool,
        nat_enabled: bool,
        switch: Option<Arc<Mutex<VirtualSwitch>>>,
        graph_index: u8,
    ) -> Self {
        let lan_mac = generate_mac(vm_id, 0);
        let nat_mac = generate_mac(vm_id, 1);

        let lan_rx = if lan_enabled {
            if let Some(ref sw) = switch {
                Some(sw.lock().unwrap().add_port(lan_mac))
            } else { None }
        } else { None };

        let slirp = SlirpDevice::new(vm_index);

        Self {
            vm_id: vm_id.to_string(),
            vm_index,
            lan_mac,
            lan_rx,
            lan_pending_tx: Vec::new(),
            nat_mac,
            slirp,
            dhcp_lan: DhcpServer::for_lan(graph_index, vm_index, lan_mac),
            dhcp_nat: DhcpServer::for_nat(vm_index, nat_mac),
            dns: DnsForwarder::default(),
            switch,
        }
    }

    // Guest OS sends frame via eth0 (LAN)
    pub fn lan_send(&mut self, frame: Vec<u8>) {
        // Check DHCP first
        if let Some(reply) = self.dhcp_lan.handle(&frame) {
            self.lan_pending_tx.push(reply);
            return;
        }
        // Check DNS
        if let Some(reply) = self.dns.handle(&frame) {
            self.lan_pending_tx.push(reply);
            return;
        }
        // Forward to switch
        if let Some(sw) = &self.switch {
            sw.lock().unwrap().deliver(&self.lan_mac, frame);
        }
    }

    // Guest OS sends frame via eth1 (NAT)
    pub fn nat_send(&mut self, frame: Vec<u8>) {
        // Check DHCP first
        if let Some(reply) = self.dhcp_nat.handle(&frame) {
            self.lan_pending_tx.push(reply);
            return;
        }
        // Check DNS
        if let Some(reply) = self.dns.handle(&frame) {
            self.lan_pending_tx.push(reply);
            return;
        }
        // Hand to smoltcp NAT
        self.slirp.guest_send(frame);
    }

    // Poll: drain switch rx, poll NAT stack, collect frames for guest
    pub fn poll(&mut self) -> Vec<(NicId, Vec<u8>)> {
        let mut frames = Vec::new();

        // Drain LAN pending (DHCP/DNS replies)
        for f in self.lan_pending_tx.drain(..) {
            frames.push((NicId::Lan, f));
        }

        // Drain frames from switch (other VMs sent us something)
        if let Some(rx) = &self.lan_rx {
            if let Ok(rx) = rx.try_lock() {
                while let Ok(frame) = rx.try_recv() {
                    frames.push((NicId::Lan, frame));
                }
            }
        }

        // Poll NAT
        self.slirp.poll();
        while let Some(f) = self.slirp.guest_recv() {
            frames.push((NicId::Nat, f));
        }

        frames
    }

    pub fn add_port_forward(&mut self, host_port: u16, guest_port: u16, proto: &str) {
        self.slirp.add_port_forward(host_port, guest_port, proto);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NicId { Lan, Nat }

fn generate_mac(vm_id: &str, nic: u8) -> [u8; 6] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    vm_id.hash(&mut h);
    let hash = h.finish();
    [0x52, 0x54,
     ((hash >> 32) & 0xFF) as u8,
     ((hash >> 16) & 0xFF) as u8,
     ((hash >> 8)  & 0xFF) as u8,
     (hash & 0xFF) as u8 ^ nic]
}
