// virtio-net: two NICs per VM
// eth0 → LAN (virtual switch between VMs in same graph)
// eth1 → NAT (smoltcp userspace internet)

use std::sync::{Arc, Mutex};
use crate::net::switch::VirtualSwitch;

pub struct NetDevice {
    pub mac_lan: [u8; 6],
    pub mac_nat: [u8; 6],
    pub lan_enabled: bool,
    pub nat_enabled: bool,
    pub switch: Option<Arc<Mutex<VirtualSwitch>>>,
    lan_rx: Vec<Vec<u8>>,
    nat_rx: Vec<Vec<u8>>,
}

impl NetDevice {
    pub fn new(vm_id: &str, lan: bool, nat: bool) -> Self {
        Self {
            mac_lan: generate_mac(vm_id, 0),
            mac_nat: generate_mac(vm_id, 1),
            lan_enabled: lan,
            nat_enabled: nat,
            switch: None,
            lan_rx: Vec::new(),
            nat_rx: Vec::new(),
        }
    }

    pub fn attach_switch(&mut self, switch: Arc<Mutex<VirtualSwitch>>) {
        self.switch = Some(switch);
    }

    // Guest OS sends frame via eth0 (LAN)
    pub fn lan_send(&mut self, frame: Vec<u8>) {
        if let Some(sw) = &self.switch {
            let mut sw = sw.lock().unwrap();
            sw.deliver(&self.mac_lan, frame);
        }
    }

    // Guest OS sends frame via eth1 (NAT/internet)
    pub fn nat_send(&mut self, frame: Vec<u8>) {
        // Hand to smoltcp NAT stack in graph_net
        // In full impl: pass to SlirpDevice
        drop(frame);
    }

    // Called by switch when a frame is destined for this VM's eth0
    pub fn lan_recv(&mut self, frame: Vec<u8>) {
        self.lan_rx.push(frame);
    }

    // Called by NAT stack when a response arrives for eth1
    pub fn nat_recv(&mut self, frame: Vec<u8>) {
        self.nat_rx.push(frame);
    }

    pub fn drain_lan_rx(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.lan_rx)
    }

    pub fn drain_nat_rx(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.nat_rx)
    }
}

fn generate_mac(vm_id: &str, nic_index: u8) -> [u8; 6] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut h = DefaultHasher::new();
    vm_id.hash(&mut h);
    let hash = h.finish();

    [
        0x52, // locally administered, unicast
        0x54,
        ((hash >> 32) & 0xFF) as u8,
        ((hash >> 16) & 0xFF) as u8,
        ((hash >> 8)  & 0xFF) as u8,
        (hash & 0xFF) as u8 ^ nic_index,
    ]
}
