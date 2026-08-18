use super::{FrameRect, HvBackend, MachineConfig, VmState};

pub fn is_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        // Hypervisor.framework available on macOS 10.15+ with Intel, 11+ with Apple Silicon
        // Check via sysctl
        let output = std::process::Command::new("sysctl")
            .args(["-n", "kern.hv_support"])
            .output();
        matches!(output, Ok(o) if o.stdout.starts_with(b"1"))
    }
    #[cfg(not(target_os = "macos"))]
    false
}

pub struct HvfBackend {
    state: VmState,
    width: u32,
    height: u32,
    serial_rx: Vec<u8>,
}

impl HvfBackend {
    pub fn new() -> Self {
        Self {
            state: VmState::Stopped,
            width: 1024,
            height: 768,
            serial_rx: Vec::new(),
        }
    }
}

impl HvBackend for HvfBackend {
    fn start(&mut self, config: &MachineConfig) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            // Hypervisor.framework FFI
            // Full impl: hv_vm_create(), hv_vcpu_create(), map guest memory,
            // load kernel, set vcpu state, run loop in thread
            // Reference: https://developer.apple.com/documentation/hypervisor
            self.state = VmState::Running;
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        Err("HVF: macOS only".into())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.state = VmState::Stopped;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), String> {
        self.state = VmState::Paused;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), String> {
        self.state = VmState::Running;
        Ok(())
    }

    fn send_key(&mut self, _keycode: u32, _pressed: bool) -> Result<(), String> { Ok(()) }
    fn send_mouse(&mut self, _x: i32, _y: i32, _buttons: u8) -> Result<(), String> { Ok(()) }
    fn send_serial(&mut self, _data: &[u8]) -> Result<(), String> { Ok(()) }
    fn recv_serial(&mut self) -> Option<Vec<u8>> {
        if self.serial_rx.is_empty() { None } else { Some(std::mem::take(&mut self.serial_rx)) }
    }
    fn get_dirty_rects(&mut self) -> Vec<FrameRect> { vec![] }
    fn get_frame(&mut self) -> Option<super::FrameSnapshot> { None }
    fn state(&mut self) -> VmState { self.state.clone() }
    fn framebuffer_size(&self) -> (u32, u32) { (self.width, self.height) }
}
