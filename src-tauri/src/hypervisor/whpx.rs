use super::{FrameRect, HvBackend, MachineConfig, VmState};

pub fn is_available() -> bool {
    #[cfg(target_os = "windows")]
    {
        // Check Windows Hypervisor Platform via WHvGetCapability
        // Simplified check via registry or API
        use std::os::windows::ffi::OsStrExt;
        // Full impl would call WHvGetCapability(WHvCapabilityCodeHypervisorPresent, ...)
        false // TODO: implement registry check
    }
    #[cfg(not(target_os = "windows"))]
    false
}

pub struct WhpxBackend {
    state: VmState,
    width: u32,
    height: u32,
}

impl WhpxBackend {
    pub fn new() -> Self {
        Self { state: VmState::Stopped, width: 1024, height: 768 }
    }
}

impl HvBackend for WhpxBackend {
    fn start(&mut self, _config: &MachineConfig) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            // WHvCreatePartition(), WHvSetupPartition(), WHvCreateVirtualProcessor()
            // map guest memory, load kernel, run vcpu loop
            self.state = VmState::Running;
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        Err("WHPX: Windows only".into())
    }

    fn stop(&mut self) -> Result<(), String> { self.state = VmState::Stopped; Ok(()) }
    fn pause(&mut self) -> Result<(), String> { self.state = VmState::Paused; Ok(()) }
    fn resume(&mut self) -> Result<(), String> { self.state = VmState::Running; Ok(()) }
    fn send_key(&mut self, _: u32, _: bool) -> Result<(), String> { Ok(()) }
    fn send_mouse(&mut self, _: i32, _: i32, _: u8) -> Result<(), String> { Ok(()) }
    fn send_serial(&mut self, _: &[u8]) -> Result<(), String> { Ok(()) }
    fn recv_serial(&mut self) -> Option<Vec<u8>> { None }
    fn get_dirty_rects(&mut self) -> Vec<FrameRect> { vec![] }
    fn get_frame(&mut self) -> Option<super::FrameSnapshot> { None }
    fn state(&mut self) -> VmState { self.state.clone() }
    fn framebuffer_size(&self) -> (u32, u32) { (self.width, self.height) }
}
