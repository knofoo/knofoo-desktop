use super::{FrameRect, FrameSnapshot, HvBackend, MachineConfig, VmState};

pub fn is_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        // Actually try to open KVM — /dev/kvm may exist but be inaccessible
        kvm_ioctls::Kvm::new().is_ok()
    }
    #[cfg(not(target_os = "linux"))]
    false
}

#[cfg(target_os = "linux")]
mod inner {
    use base64::Engine as _;
    use super::super::{FrameRect, HvBackend, MachineConfig, VmState};
    use kvm_ioctls::{Kvm, VmFd, VcpuFd};
    use kvm_bindings::kvm_userspace_memory_region;
    use vm_memory::{GuestAddress, GuestMemoryMmap, GuestMemory};
    use std::sync::Arc;
    use std::thread::JoinHandle;
    use std::sync::atomic::{AtomicBool, Ordering};

    const GUEST_MEM_START: u64 = 0x0;
    const BIOS_START: u64 = 0xF0000;
    const FRAMEBUFFER_ADDR: u64 = 0xF000_0000;
    const FRAMEBUFFER_W: u32 = 1024;
    const FRAMEBUFFER_H: u32 = 768;
    const FRAMEBUFFER_SIZE: usize = (FRAMEBUFFER_W * FRAMEBUFFER_H * 4) as usize;

    pub struct KvmBackend {
        state: VmState,
        kvm: Option<Kvm>,
        vm: Option<VmFd>,
        vcpu: Option<VcpuFd>,
        mem: Option<GuestMemoryMmap>,
        framebuffer: Vec<u8>,
        prev_framebuffer: Vec<u8>,
        serial_tx: Vec<u8>,
        serial_rx: Vec<u8>,
        run_flag: Arc<AtomicBool>,
        vcpu_thread: Option<JoinHandle<()>>,
        width: u32,
        height: u32,
    }

    impl KvmBackend {
        pub fn new() -> Self {
            Self {
                state: VmState::Stopped,
                kvm: None,
                vm: None,
                vcpu: None,
                mem: None,
                framebuffer: vec![0u8; FRAMEBUFFER_SIZE],
                prev_framebuffer: vec![0u8; FRAMEBUFFER_SIZE],
                serial_tx: Vec::new(),
                serial_rx: Vec::new(),
                run_flag: Arc::new(AtomicBool::new(false)),
                vcpu_thread: None,
                width: FRAMEBUFFER_W,
                height: FRAMEBUFFER_H,
            }
        }

        fn setup_memory(&mut self, ram_mb: u64) -> Result<(), String> {
            let ram_size = ram_mb * 1024 * 1024;
            let mem = GuestMemoryMmap::<()>::from_ranges(&[
                (GuestAddress(GUEST_MEM_START), ram_size as usize),
            ]).map_err(|e| e.to_string())?;

            let vm = self.vm.as_ref().ok_or("no vm")?;
            let host_addr = mem.get_host_address(GuestAddress(GUEST_MEM_START))
                .map_err(|e| e.to_string())?;

            let mem_region = kvm_userspace_memory_region {
                slot: 0,
                guest_phys_addr: GUEST_MEM_START,
                memory_size: ram_size,
                userspace_addr: host_addr as u64,
                flags: 0,
            };
            unsafe {
                vm.set_user_memory_region(mem_region).map_err(|e| e.to_string())?;
            }
            self.mem = Some(mem);
            Ok(())
        }

        fn load_kernel(&self, iso_path: &str) -> Result<(), String> {
            use linux_loader::loader::KernelLoader;
            use linux_loader::loader::bzimage::BzImage;
            let mem = self.mem.as_ref().ok_or("no mem")?;
            let mut f = std::fs::File::open(iso_path).map_err(|e| e.to_string())?;
            BzImage::load(
                mem,
                None,
                &mut f,
                Some(GuestAddress(0x100000)),
            ).map_err(|e| format!("{:?}", e))?;
            Ok(())
        }

        fn setup_vcpu_regs(&self) -> Result<(), String> {
            use kvm_bindings::kvm_regs;
            let vcpu = self.vcpu.as_ref().ok_or("no vcpu")?;

            let mut regs = vcpu.get_regs().map_err(|e| e.to_string())?;
            regs.rip = 0x100000;
            regs.rsp = 0x8000;
            regs.rflags = 0x2;
            vcpu.set_regs(&regs).map_err(|e| e.to_string())?;

            let mut sregs = vcpu.get_sregs().map_err(|e| e.to_string())?;
            sregs.cs.base = 0;
            sregs.cs.selector = 0;
            vcpu.set_sregs(&sregs).map_err(|e| e.to_string())?;

            Ok(())
        }
    }

    impl HvBackend for KvmBackend {
        fn start(&mut self, config: &MachineConfig) -> Result<(), String> {
            self.state = VmState::Starting;

            let kvm = Kvm::new().map_err(|e| e.to_string())?;
            let vm = kvm.create_vm().map_err(|e| e.to_string())?;
            self.kvm = Some(kvm);
            self.vm = Some(vm);

            self.setup_memory(config.ram_mb)?;

            let vcpu = self.vm.as_ref().unwrap()
                .create_vcpu(0).map_err(|e| e.to_string())?;
            self.vcpu = Some(vcpu);

            if let Some(iso) = &config.iso_path {
                self.load_kernel(iso).unwrap_or_else(|e| {
                    eprintln!("kernel load warn: {}", e);
                });
            }

            self.setup_vcpu_regs()?;

            self.run_flag.store(true, Ordering::SeqCst);
            self.state = VmState::Running;

            Ok(())
        }

        fn stop(&mut self) -> Result<(), String> {
            self.run_flag.store(false, Ordering::SeqCst);
            self.state = VmState::Stopped;
            Ok(())
        }

        fn pause(&mut self) -> Result<(), String> {
            self.run_flag.store(false, Ordering::SeqCst);
            self.state = VmState::Paused;
            Ok(())
        }

        fn resume(&mut self) -> Result<(), String> {
            self.run_flag.store(true, Ordering::SeqCst);
            self.state = VmState::Running;
            Ok(())
        }

        fn send_key(&mut self, keycode: u32, pressed: bool) -> Result<(), String> {
            // Inject key via KVM I/O port 0x60 (PS/2 keyboard)
            // In full impl: write scancode to i8042 emulation
            let _ = (keycode, pressed);
            Ok(())
        }

        fn send_mouse(&mut self, x: i32, y: i32, buttons: u8) -> Result<(), String> {
            let _ = (x, y, buttons);
            Ok(())
        }

        fn send_serial(&mut self, data: &[u8]) -> Result<(), String> {
            self.serial_tx.extend_from_slice(data);
            Ok(())
        }

        fn recv_serial(&mut self) -> Option<Vec<u8>> {
            if self.serial_rx.is_empty() { return None; }
            Some(std::mem::take(&mut self.serial_rx))
        }

        fn get_dirty_rects(&mut self) -> Vec<FrameRect> {
            let vm = match &self.vm { Some(v) => v, None => return vec![] };

            // Get dirty bitmap from KVM
            let dirty = match vm.get_dirty_log(0, self.framebuffer.len()) {
                Ok(d) => d,
                Err(_) => return vec![],
            };

            let page_size = 4096usize;
            let bytes_per_row = (self.width * 4) as usize;
            let mut rects = Vec::new();

            for (page_idx, &bitmap_byte) in dirty.iter().enumerate() {
                for bit in 0..8 {
                    if bitmap_byte & (1 << bit) != 0 {
                        let page_addr = (page_idx * 8 + bit) * page_size;
                        if page_addr + page_size > self.framebuffer.len() { continue; }

                        let row = (page_addr / bytes_per_row) as u32;
                        rects.push(FrameRect {
                            x: 0,
                            y: row,
                            w: self.width,
                            h: (page_size / bytes_per_row) as u32,
                            data: base64::engine::general_purpose::STANDARD
                                .encode(&self.framebuffer[page_addr..page_addr + page_size]),
                        });
                    }
                }
            }

            rects
        }

        fn get_frame(&mut self) -> Option<super::super::FrameSnapshot> { None }

        fn state(&mut self) -> VmState {
            self.state.clone()
        }

        fn framebuffer_size(&self) -> (u32, u32) {
            (self.width, self.height)
        }
    }
}

#[cfg(target_os = "linux")]
pub use inner::KvmBackend;

#[cfg(not(target_os = "linux"))]
pub struct KvmBackend;

#[cfg(not(target_os = "linux"))]
impl KvmBackend {
    pub fn new() -> Self { Self }
}

#[cfg(not(target_os = "linux"))]
impl HvBackend for KvmBackend {
    fn start(&mut self, _: &MachineConfig) -> Result<(), String> { Err("KVM: Linux only".into()) }
    fn stop(&mut self) -> Result<(), String> { Ok(()) }
    fn pause(&mut self) -> Result<(), String> { Ok(()) }
    fn resume(&mut self) -> Result<(), String> { Ok(()) }
    fn send_key(&mut self, _: u32, _: bool) -> Result<(), String> { Ok(()) }
    fn send_mouse(&mut self, _: i32, _: i32, _: u8) -> Result<(), String> { Ok(()) }
    fn send_serial(&mut self, _: &[u8]) -> Result<(), String> { Ok(()) }
    fn recv_serial(&mut self) -> Option<Vec<u8>> { None }
    fn get_dirty_rects(&mut self) -> Vec<FrameRect> { vec![] }
    fn get_frame(&mut self) -> Option<FrameSnapshot> { None }
    fn state(&mut self) -> VmState { VmState::Stopped }
    fn framebuffer_size(&self) -> (u32, u32) { (1024, 768) }
}
