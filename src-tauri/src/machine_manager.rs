// Machine manager: lifecycle for all running VMs.
// One manager per app, stored in Tauri state.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use parking_lot::RwLock;

use crate::hypervisor::{create_backend, MachineConfig, SharedBackend, VmState, VmStatus};
use crate::virtio::gpu::GpuDevice;
use crate::virtio::serial::SerialDevice;
use crate::virtio::input::{InputDevice, InputEvent};
use crate::net::switch::SwitchRegistry;
use crate::net::graph_net::VmNetwork;

pub struct VmSession {
    pub config: MachineConfig,
    pub backend: SharedBackend,
    pub gpu: Arc<Mutex<GpuDevice>>,
    pub serial: Arc<Mutex<SerialDevice>>,
    pub input: InputDevice,
    pub network: Option<VmNetwork>,
}

pub struct MachineManager {
    sessions: RwLock<HashMap<String, VmSession>>,
    switches: Mutex<SwitchRegistry>,
    graph_vm_index: Mutex<HashMap<String, u8>>, // graph_id → next vm_index
}

impl MachineManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            switches: Mutex::new(SwitchRegistry::new()),
            graph_vm_index: Mutex::new(HashMap::new()),
        }
    }

    pub fn start(&self, config: MachineConfig) -> Result<(), String> {
        let id = config.id.clone();
        eprintln!("[MachineManager::start] id={}", id);

        if self.sessions.read().contains_key(&id) {
            return Err(format!("VM {} already running", id));
        }
        eprintln!("[MachineManager::start] session check passed");
        eprintln!("[MachineManager::start] creating GPU/serial/input...");

        let gpu    = Arc::new(Mutex::new(GpuDevice::new()));
        eprintln!("[MachineManager::start] gpu created");
        let serial = Arc::new(Mutex::new(SerialDevice::new()));
        eprintln!("[MachineManager::start] serial created");
        let (fb_w, fb_h) = { let g = gpu.lock().unwrap(); (g.width, g.height) };
        let input  = InputDevice::new(fb_w, fb_h);
        eprintln!("[MachineManager::start] input created, network setup...");

        // Network setup
        let network = if config.network.lan || config.network.internet {
            let vm_index = self.next_vm_index(&config.id);
            let graph_index = 0u8; // TODO: derive from graph_id when graph context available

            let switch = if config.network.lan {
                Some(self.switches.lock().unwrap().get_or_create(&config.id))
            } else { None };

            let mut net = VmNetwork::new(
                &id,
                vm_index,
                config.network.lan,
                config.network.internet,
                switch,
                graph_index,
            );

            for pf in &config.network.port_forwards {
                net.add_port_forward(pf.host, pf.guest, &pf.proto);
            }

            Some(net)
        } else { None };

        eprintln!("[MachineManager::start] creating backend...");
        let mut backend = create_backend(Arc::clone(&serial));
        eprintln!("[MachineManager::start] calling backend.start()...");
        backend.start(&config)?;
        eprintln!("[MachineManager::start] backend.start() returned OK");

        let session = VmSession {
            config,
            backend: Arc::new(parking_lot::Mutex::new(backend)),
            gpu,
            serial,
            input,
            network,
        };

        self.sessions.write().insert(id, session);
        Ok(())
    }

    pub fn stop(&self, id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write();
        if let Some(session) = sessions.get(id) {
            session.backend.lock().stop()?;
        }
        sessions.remove(id);
        Ok(())
    }

    pub fn pause(&self, id: &str) -> Result<(), String> {
        let backend = { let g = self.sessions.read(); g.get(id).map(|s| Arc::clone(&s.backend)).ok_or("VM not found")? };
        let result = backend.lock().pause();
        result
    }

    pub fn resume(&self, id: &str) -> Result<(), String> {
        let backend = { let g = self.sessions.read(); g.get(id).map(|s| Arc::clone(&s.backend)).ok_or("VM not found")? };
        let result = backend.lock().resume();
        result
    }

    pub fn send_input(&self, id: &str, event: InputEvent) -> Result<(), String> {
        let backend = {
            let sessions = self.sessions.read();
            sessions.get(id).map(|s| Arc::clone(&s.backend)).ok_or("VM not found")?
        };
        // Query live framebuffer size — initial InputDevice has stale defaults
        // before the guest first calls VNC ServerInit.
        let (fb_w, fb_h) = backend.lock().framebuffer_size();
        let live_input = crate::virtio::input::InputDevice::new(fb_w, fb_h);
        let vm_event = live_input.translate(event);
        if let Some(vm_event) = vm_event {
            use crate::virtio::input::VmInputKind;
            let mut b = backend.lock();
            match vm_event.kind {
                VmInputKind::KeyDown(kc)  => b.send_key(kc, true)?,
                VmInputKind::KeyUp(kc)    => b.send_key(kc, false)?,
                // 0x00 — move only, uses stored button state
                VmInputKind::MouseMove(x, y) => b.send_mouse(x, y, 0)?,
                // High bit signals "set/clear bit, then send": 0x80 = press, 0x40 = release.
                // Backend ANDs out flags before sending.
                VmInputKind::MouseButton { x, y, button, pressed } => {
                    let cmd = if pressed { 0x80 | (1u8 << button) } else { 0x40 | (1u8 << button) };
                    b.send_mouse(x, y, cmd)?;
                }
                // 0x08 = wheel marker (one-shot, doesn't change held buttons)
                VmInputKind::MouseWheel { dx, dy } => b.send_mouse(dx, dy, 0x08)?,
            }
        }
        Ok(())
    }

    pub fn send_serial(&self, id: &str, data: &[u8]) -> Result<(), String> {
        let serial = { let g = self.sessions.read(); g.get(id).map(|s| Arc::clone(&s.serial)).ok_or("VM not found")? };
        serial.lock().unwrap().send_to_guest(data);
        Ok(())
    }

    pub fn send_clipboard_to_vm(&self, id: &str, text: &str) -> Result<(), String> {
        let serial = { let g = self.sessions.read(); g.get(id).map(|s| Arc::clone(&s.serial)).ok_or("VM not found")? };
        serial.lock().unwrap().send_clipboard_to_guest(text);
        Ok(())
    }

    pub fn drain_stdout(&self, id: &str) -> Option<String> {
        let serial = { let g = self.sessions.read(); g.get(id).map(|s| Arc::clone(&s.serial))? };
        let result = serial.lock().unwrap().drain_stdout();
        result
    }

    pub fn drain_clipboard(&self, id: &str) -> Option<String> {
        let serial = { let g = self.sessions.read(); g.get(id).map(|s| Arc::clone(&s.serial))? };
        let result = serial.lock().unwrap().drain_clipboard();
        result
    }

    pub fn get_frame(&self, id: &str) -> Option<crate::hypervisor::FrameSnapshot> {
        let backend = { let g = self.sessions.read(); g.get(id).map(|s| Arc::clone(&s.backend))? };
        let result = backend.lock().get_frame();
        result
    }

    pub fn get_frame_bin(&self, id: &str) -> Vec<u8> {
        let backend = match { let g = self.sessions.read(); g.get(id).map(|s| Arc::clone(&s.backend)) } {
            Some(b) => b, None => return Vec::new(),
        };
        let result = backend.lock().get_frame_bin();
        result
    }

    pub fn get_dirty_rects(&self, id: &str) -> Vec<crate::hypervisor::FrameRect> {
        let backend = match { let g = self.sessions.read(); g.get(id).map(|s| Arc::clone(&s.backend)) } {
            Some(b) => b, None => return vec![],
        };
        let result = backend.lock().get_dirty_rects();
        result
    }

    pub fn status(&self, id: &str) -> Option<VmStatus> {
        let backend = { let g = self.sessions.read(); g.get(id).map(|s| Arc::clone(&s.backend))? };
        let mut b = backend.lock();
        let (w, h) = b.framebuffer_size();
        Some(VmStatus {
            id: id.to_string(),
            state: b.state(),
            width: w,
            height: h,
        })
    }

    pub fn list(&self) -> Vec<VmStatus> {
        let ids: Vec<String> = self.sessions.read().keys().cloned().collect();
        ids.iter().filter_map(|id| self.status(id)).collect()
    }

    pub fn diag(&self, id: &str) -> String {
        let sessions = self.sessions.read();
        match sessions.get(id) {
            None => format!("no session for '{}'", id),
            Some(s) => {
                let mut b = s.backend.lock();
                let state = b.state();
                let (w, h) = b.framebuffer_size();
                format!("session found, state={:?}, fb={}x{}", state, w, h)
            }
        }
    }

    fn next_vm_index(&self, vm_id: &str) -> u8 {
        let mut map = self.graph_vm_index.lock().unwrap();
        let idx = map.entry(vm_id.to_string()).or_insert(0);
        let result = *idx;
        *idx += 1;
        result
    }
}

// Tauri managed state wrapper
pub struct MachineState(pub Arc<MachineManager>);

impl MachineState {
    pub fn new() -> Self {
        Self(Arc::new(MachineManager::new()))
    }
    pub fn manager(&self) -> &MachineManager { &self.0 }
}
