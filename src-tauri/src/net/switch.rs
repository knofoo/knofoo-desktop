// Virtual L2 switch for LAN between VMs in the same graph.
// Graph edge A→B = A and B registered on same switch instance.
// No edge = VM isolated (switch has only one port, frames go nowhere).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type FrameSender = std::sync::mpsc::SyncSender<Vec<u8>>;
pub type FrameReceiver = Arc<Mutex<std::sync::mpsc::Receiver<Vec<u8>>>>;

const BROADCAST: [u8; 6] = [0xFF; 6];

pub struct VirtualSwitch {
    // MAC address → channel to deliver frames to that VM
    ports: HashMap<[u8; 6], FrameSender>,
    // graph_id this switch belongs to
    graph_id: String,
}

impl VirtualSwitch {
    pub fn new(graph_id: &str) -> Self {
        Self { ports: HashMap::new(), graph_id: graph_id.to_string() }
    }

    pub fn add_port(&mut self, mac: [u8; 6]) -> FrameReceiver {
        let (tx, rx) = std::sync::mpsc::sync_channel(256);
        self.ports.insert(mac, tx);
        Arc::new(Mutex::new(rx))
    }

    pub fn remove_port(&mut self, mac: &[u8; 6]) {
        self.ports.remove(mac);
    }

    pub fn deliver(&self, src_mac: &[u8; 6], frame: Vec<u8>) {
        if frame.len() < 6 { return; }
        let dst_mac: [u8; 6] = frame[0..6].try_into().unwrap_or([0; 6]);

        if dst_mac == BROADCAST {
            // Flood to all ports except sender
            for (mac, tx) in &self.ports {
                if mac != src_mac {
                    let _ = tx.send(frame.clone());
                }
            }
        } else {
            // Unicast
            if let Some(tx) = self.ports.get(&dst_mac) {
                let _ = tx.send(frame);
            }
        }
    }

    pub fn graph_id(&self) -> &str { &self.graph_id }
    pub fn port_count(&self) -> usize { self.ports.len() }
}

// Global switch registry: graph_id → switch
// Allows VMs added to the same graph to share a switch
pub struct SwitchRegistry {
    switches: HashMap<String, Arc<Mutex<VirtualSwitch>>>,
}

impl SwitchRegistry {
    pub fn new() -> Self { Self { switches: HashMap::new() } }

    pub fn get_or_create(&mut self, graph_id: &str) -> Arc<Mutex<VirtualSwitch>> {
        self.switches
            .entry(graph_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(VirtualSwitch::new(graph_id))))
            .clone()
    }

    pub fn remove_graph(&mut self, graph_id: &str) {
        self.switches.remove(graph_id);
    }
}
