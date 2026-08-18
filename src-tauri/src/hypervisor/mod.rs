use std::sync::Arc;
use parking_lot::Mutex;

pub mod kvm;
pub mod hvf;
pub mod whpx;
pub mod fallback;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BootMode {
    Auto,
    Uefi,
    Bios,
    Disk,
}

impl Default for BootMode {
    fn default() -> Self { BootMode::Auto }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MachineConfig {
    pub id: String,
    pub iso_path: Option<String>,
    pub disk_path: String,
    pub ram_mb: u64,
    pub cpus: u32,
    pub shared_folder: Option<String>,
    #[serde(default)]
    pub boot_mode: BootMode,
    pub network: NetworkConfig,
    pub input: InputConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkConfig {
    pub lan: bool,
    pub internet: bool,
    pub port_forwards: Vec<PortForward>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortForward {
    pub host: u16,
    pub guest: u16,
    pub proto: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputConfig {
    pub keyboard_passthrough: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self { lan: true, internet: true, port_forwards: vec![] }
    }
}

impl Default for InputConfig {
    fn default() -> Self {
        Self { keyboard_passthrough: true }
    }
}

// Full framebuffer snapshot returned by vm_get_frame command.
// data is base64-encoded RGBA. None when nothing changed since last call.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FrameSnapshot {
    pub w: u32,
    pub h: u32,
    pub data: String, // base64 RGBA, row-major
}

// Keep FrameRect for KVM dirty-bitmap path (unused by fallback now)
#[derive(Debug, Clone, serde::Serialize)]
pub struct FrameRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub data: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VmStatus {
    pub id: String,
    pub state: VmState,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VmState {
    Stopped,
    Starting,
    Running,
    Paused,
    Error,
}

pub trait HvBackend: Send + Sync {
    fn start(&mut self, config: &MachineConfig) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
    fn pause(&mut self) -> Result<(), String>;
    fn resume(&mut self) -> Result<(), String>;
    fn send_key(&mut self, keycode: u32, pressed: bool) -> Result<(), String>;
    fn send_mouse(&mut self, x: i32, y: i32, buttons: u8) -> Result<(), String>;
    fn send_serial(&mut self, data: &[u8]) -> Result<(), String>;
    fn recv_serial(&mut self) -> Option<Vec<u8>>;
    fn get_dirty_rects(&mut self) -> Vec<FrameRect>;
    // Returns full framebuffer if changed since last call, else None.
    fn get_frame(&mut self) -> Option<FrameSnapshot>;
    // Binary variant: [w:u32 LE | h:u32 LE | rgba ...]. Empty Vec means no new frame.
    fn get_frame_bin(&mut self) -> Vec<u8> { Vec::new() }
    fn state(&mut self) -> VmState;
    fn framebuffer_size(&self) -> (u32, u32);
}

pub type SharedSerial = Arc<std::sync::Mutex<crate::virtio::serial::SerialDevice>>;

pub fn create_backend(serial: SharedSerial) -> Box<dyn HvBackend> {
    // KVM/HVF/WHPX backends are stubs — only FallbackBackend (QEMU+VNC) is fully implemented.
    Box::new(fallback::FallbackBackend::new(serial))
}

pub type SharedBackend = Arc<Mutex<Box<dyn HvBackend>>>;
