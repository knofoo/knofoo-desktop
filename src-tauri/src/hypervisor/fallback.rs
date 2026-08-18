// Fallback: spawn QEMU, connect a minimal RFB client to its VNC socket,
// push framebuffer updates into a shared GpuDevice.

use super::{BootMode, FrameRect, HvBackend, MachineConfig, VmState};

#[derive(Debug, Clone, Copy, PartialEq)]
enum IsoBootKind {
    UefiHybrid,    // MBR/GPT with EFI partition — extract ESP
    UefiElTorito,  // El Torito UEFI entry only — OVMF + cdrom
    BiosOnly,      // El Torito BIOS only — SeaBIOS + cdrom
    Unknown,       // Fallback to BIOS
}
use crate::virtio::gpu::GpuDevice;
use crate::virtio::serial::SerialDevice;
use std::process::{Child, Command, Stdio};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::time::Duration;

pub struct FallbackBackend {
    state: VmState,
    child: Option<Child>,
    vnc_port: u16,
    agent_port: u16,
    gpu: Arc<Mutex<GpuDevice>>,
    // Shared agent channel (virtio-serial). Filled by machine_manager and bridged
    // to the guest by start_agent_bridge. host→guest via next_tx, guest→host via recv_from_guest.
    serial: Arc<Mutex<SerialDevice>>,
    vnc_thread: Option<std::thread::JoinHandle<()>>,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
    error_msg: Arc<Mutex<Option<String>>>,
    input_stream: Arc<Mutex<Option<TcpStream>>>,
    // Write half of the agent virtio-serial socket (host→guest).
    agent_stream: Arc<Mutex<Option<TcpStream>>>,
    vnc_connected: Arc<std::sync::atomic::AtomicBool>,
    button_mask: u8,
}

impl FallbackBackend {
    fn pick_vnc_port() -> u16 {
        eprintln!("[pick_vnc_port] start");
        for p in 5910u16..5950 {
            if std::net::TcpListener::bind(("127.0.0.1", p)).is_ok() {
                eprintln!("[pick_vnc_port] picked {}", p);
                return p;
            }
        }
        eprintln!("[pick_vnc_port] fallback 5910");
        5910
    }

    fn pick_agent_port() -> u16 {
        for p in 6010u16..6050 {
            if std::net::TcpListener::bind(("127.0.0.1", p)).is_ok() {
                return p;
            }
        }
        6010
    }

    pub fn new(serial: Arc<Mutex<SerialDevice>>) -> Self {
        eprintln!("[FallbackBackend::new] entered");
        Self {
            state: VmState::Stopped,
            child: None,
            vnc_port: Self::pick_vnc_port(),
            agent_port: Self::pick_agent_port(),
            gpu: Arc::new(Mutex::new(GpuDevice::new())),
            serial,
            vnc_thread: None,
            stop_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            error_msg: Arc::new(Mutex::new(None)),
            input_stream: Arc::new(Mutex::new(None)),
            agent_stream: Arc::new(Mutex::new(None)),
            vnc_connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            button_mask: 0,
        }
    }

    fn qemu_binary() -> Option<std::path::PathBuf> {
        let app_bin = dirs_next::data_dir()
            .map(|d| d.join("knofoo").join("bin").join(qemu_binary_name()));
        if let Some(ref p) = app_bin {
            if p.exists() { return app_bin; }
        }
        which::which(qemu_binary_name()).ok()
    }

    fn ovmf_path() -> Option<std::path::PathBuf> {
        let candidates = [
            "/usr/share/edk2/x64/OVMF.4m.fd",
            "/usr/share/edk2/x64/OVMF.fd",
            "/usr/share/ovmf/OVMF.fd",
            "/usr/share/qemu/OVMF.fd",
            "/usr/share/edk2-ovmf/OVMF.fd",
        ];
        candidates.iter()
            .map(std::path::Path::new)
            .find(|p| p.exists())
            .map(|p| p.to_path_buf())
    }

    fn ovmf_split() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        let code_candidates = [
            "/usr/share/edk2/x64/OVMF_CODE.4m.fd",
            "/usr/share/edk2/x64/OVMF_CODE.fd",
            "/usr/share/OVMF/OVMF_CODE.fd",
            "/usr/share/edk2-ovmf/OVMF_CODE.fd",
        ];
        let vars_candidates = [
            "/usr/share/edk2/x64/OVMF_VARS.4m.fd",
            "/usr/share/edk2/x64/OVMF_VARS.fd",
            "/usr/share/OVMF/OVMF_VARS.fd",
            "/usr/share/edk2-ovmf/OVMF_VARS.fd",
        ];
        let code = code_candidates.iter().map(std::path::Path::new).find(|p| p.exists())?;
        let vars = vars_candidates.iter().map(std::path::Path::new).find(|p| p.exists())?;
        Some((code.to_path_buf(), vars.to_path_buf()))
    }

    fn vm_vars_path(vm_id: &str) -> Option<std::path::PathBuf> {
        let dir = dirs_next::data_dir()?.join("knofoo").join("ovmf");
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir.join(format!("{}.vars.fd", vm_id)))
    }

    fn vm_esp_dir(vm_id: &str) -> Option<std::path::PathBuf> {
        let dir = dirs_next::data_dir()?.join("knofoo").join("esp").join(vm_id);
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }

    /// Inspect an ISO and decide how it should be booted.
    /// Priority:
    ///   1. MBR partition table contains a 0xEF (EFI) partition → UefiHybrid
    ///   2. El Torito boot catalog has a UEFI (0xEF) entry → UefiElTorito
    ///   3. El Torito has only a BIOS (0x00) entry → BiosOnly
    ///   4. Fall back to BiosOnly (most legacy ISOs)
    fn detect_iso_boot_kind(iso: &std::path::Path) -> IsoBootKind {
        use std::io::{Read, Seek, SeekFrom};
        let mut f = match std::fs::File::open(iso) {
            Ok(f) => f,
            Err(_) => return IsoBootKind::Unknown,
        };

        // 1. MBR partition with type 0xEF → hybrid UEFI
        let mut mbr = [0u8; 512];
        if f.read_exact(&mut mbr).is_ok() && mbr[510] == 0x55 && mbr[511] == 0xAA {
            for i in 0..4 {
                let off = 0x1BE + i * 16;
                if mbr[off + 4] == 0xEF {
                    return IsoBootKind::UefiHybrid;
                }
            }
        }

        // 2/3. Boot Record Volume Descriptor at LBA 17 (offset 0x8800)
        if f.seek(SeekFrom::Start(0x8800)).is_ok() {
            let mut vd = [0u8; 2048];
            if f.read_exact(&mut vd).is_ok()
                && vd[0] == 0
                && &vd[1..6] == b"CD001"
                && vd[7..39].starts_with(b"EL TORITO SPECIFICATION")
            {
                let cat_lba = u32::from_le_bytes([vd[71], vd[72], vd[73], vd[74]]);
                if f.seek(SeekFrom::Start(cat_lba as u64 * 2048)).is_ok() {
                    let mut cat = [0u8; 2048];
                    if f.read_exact(&mut cat).is_ok() {
                        // Validation entry at offset 0: byte[1] = platform
                        let default_platform = cat[1];
                        let mut has_uefi = default_platform == 0xEF;
                        let mut has_bios = default_platform == 0x00;
                        // Walk section headers (offset 64+)
                        let mut off = 64usize;
                        while off + 32 <= cat.len() {
                            let header_id = cat[off];
                            if header_id == 0 || header_id == 0xFF { break; }
                            let platform = cat[off + 1];
                            if platform == 0xEF { has_uefi = true; }
                            if platform == 0x00 { has_bios = true; }
                            // 0x90 = section header continues, 0x91 = final
                            if header_id == 0x91 { break; }
                            off += 32;
                            // Skip the entries that belong to this section
                            // (each is 32B, count is bytes 2..4 LE)
                            let count = u16::from_le_bytes([cat[off - 32 + 2], cat[off - 32 + 3]]) as usize;
                            off += count * 32;
                        }
                        if has_uefi { return IsoBootKind::UefiElTorito; }
                        if has_bios { return IsoBootKind::BiosOnly; }
                    }
                }
            }
        }

        IsoBootKind::Unknown
    }

    /// Cache key for ESP extraction — based on ISO path + size + mtime so a
    /// modified ISO triggers re-extraction.
    fn iso_cache_key(iso: &std::path::Path) -> Option<String> {
        let meta = std::fs::metadata(iso).ok()?;
        let mtime = meta.modified().ok()?
            .duration_since(std::time::UNIX_EPOCH).ok()?
            .as_secs();
        Some(format!("{}-{}-{}", iso.display(), meta.len(), mtime))
    }

    /// Extract the EFI System Partition from a hybrid ISO (MBR + EFI FAT).
    /// Reads MBR, finds 0xEF partition, parses FAT32, copies EFI/ tree to `dest_dir`.
    /// Returns true on success.
    fn extract_esp_from_iso(iso: &std::path::Path, dest_dir: &std::path::Path) -> bool {
        use std::io::{Read, Seek, SeekFrom};
        let mut f = match std::fs::File::open(iso) { Ok(f) => f, Err(_) => return false };

        // MBR partition table at offset 0x1BE, 16 bytes per entry, 4 entries
        let mut mbr = [0u8; 512];
        if f.read_exact(&mut mbr).is_err() { return false; }
        let mut efi_lba: u32 = 0;
        let mut efi_secs: u32 = 0;
        for i in 0..4 {
            let off = 0x1BE + i * 16;
            let ptype = mbr[off + 4];
            if ptype == 0xEF {
                efi_lba = u32::from_le_bytes([mbr[off+8], mbr[off+9], mbr[off+10], mbr[off+11]]);
                efi_secs = u32::from_le_bytes([mbr[off+12], mbr[off+13], mbr[off+14], mbr[off+15]]);
                break;
            }
        }
        if efi_lba == 0 || efi_secs == 0 { return false; }

        let part_offset = efi_lba as u64 * 512;
        if f.seek(SeekFrom::Start(part_offset)).is_err() { return false; }
        let mut bs = [0u8; 512];
        if f.read_exact(&mut bs).is_err() { return false; }

        let bytes_per_sec = u16::from_le_bytes([bs[11], bs[12]]) as u32;
        let sec_per_cluster = bs[13] as u32;
        let reserved_secs = u16::from_le_bytes([bs[14], bs[15]]) as u32;
        let n_fats = bs[16] as u32;
        let sec_per_fat = u32::from_le_bytes([bs[36], bs[37], bs[38], bs[39]]);
        let root_cluster = u32::from_le_bytes([bs[44], bs[45], bs[46], bs[47]]);
        let data_start_sec = reserved_secs + n_fats * sec_per_fat;
        let cluster_size = bytes_per_sec * sec_per_cluster;

        let read_cluster = |f: &mut std::fs::File, cl: u32| -> Option<Vec<u8>> {
            let off = part_offset + (data_start_sec + (cl - 2) * sec_per_cluster) as u64 * bytes_per_sec as u64;
            f.seek(SeekFrom::Start(off)).ok()?;
            let mut buf = vec![0u8; cluster_size as usize];
            f.read_exact(&mut buf).ok()?;
            Some(buf)
        };
        let read_fat_chain = |f: &mut std::fs::File, start: u32| -> Vec<u32> {
            let mut chain = vec![start];
            loop {
                let last = *chain.last().unwrap();
                let fat_off = part_offset + reserved_secs as u64 * bytes_per_sec as u64 + last as u64 * 4;
                if f.seek(SeekFrom::Start(fat_off)).is_err() { break; }
                let mut b = [0u8; 4];
                if f.read_exact(&mut b).is_err() { break; }
                let nxt = u32::from_le_bytes(b) & 0x0FFF_FFFF;
                if nxt >= 0x0FFF_FFF8 || nxt == 0 || nxt == last { break; }
                chain.push(nxt);
                if chain.len() > 1_000_000 { break; }
            }
            chain
        };
        let read_file = |f: &mut std::fs::File, start: u32, size: u32| -> Option<Vec<u8>> {
            let mut data = Vec::with_capacity(size as usize);
            for c in read_fat_chain(f, start) {
                data.extend_from_slice(&read_cluster(f, c)?);
            }
            data.truncate(size as usize);
            Some(data)
        };

        fn parse_dir(data: &[u8]) -> Vec<(String, u8, u32, u32)> {
            let mut out = Vec::new();
            let mut lfn_acc: Vec<u16> = Vec::new();
            for chunk in data.chunks_exact(32) {
                if chunk[0] == 0 { break; }
                if chunk[0] == 0xE5 { lfn_acc.clear(); continue; }
                let attr = chunk[11];
                if attr == 0x0F {
                    // LFN entry — collect (we just use 8.3 below for simplicity)
                    continue;
                }
                let name8 = std::str::from_utf8(&chunk[0..8]).unwrap_or("").trim_end();
                let ext3  = std::str::from_utf8(&chunk[8..11]).unwrap_or("").trim_end();
                let mut full = name8.to_string();
                if !ext3.is_empty() { full.push('.'); full.push_str(ext3); }
                let cl_hi = u16::from_le_bytes([chunk[20], chunk[21]]) as u32;
                let cl_lo = u16::from_le_bytes([chunk[26], chunk[27]]) as u32;
                let cl = (cl_hi << 16) | cl_lo;
                let sz = u32::from_le_bytes([chunk[28], chunk[29], chunk[30], chunk[31]]);
                out.push((full, attr, cl, sz));
                lfn_acc.clear();
            }
            out
        }

        fn copy_recursive(
            f: &mut std::fs::File,
            entries: Vec<(String, u8, u32, u32)>,
            dest: &std::path::Path,
            read_chain: &dyn Fn(&mut std::fs::File, u32) -> Vec<u32>,
            read_clu: &dyn Fn(&mut std::fs::File, u32) -> Option<Vec<u8>>,
        ) -> bool {
            for (name, attr, cl, sz) in entries {
                if name.starts_with('.') || name.is_empty() { continue; }
                if attr & 0x10 != 0 {
                    // dir
                    let mut data = Vec::new();
                    for c in read_chain(f, cl) {
                        if let Some(b) = read_clu(f, c) { data.extend_from_slice(&b); } else { return false; }
                    }
                    let sub_entries = parse_dir(&data);
                    let sub_dest = dest.join(&name);
                    if std::fs::create_dir_all(&sub_dest).is_err() { return false; }
                    if !copy_recursive(f, sub_entries, &sub_dest, read_chain, read_clu) { return false; }
                } else {
                    // file
                    let mut data = Vec::with_capacity(sz as usize);
                    for c in read_chain(f, cl) {
                        if let Some(b) = read_clu(f, c) { data.extend_from_slice(&b); } else { return false; }
                    }
                    data.truncate(sz as usize);
                    let out_path = dest.join(&name);
                    if std::fs::write(&out_path, &data).is_err() { return false; }
                }
            }
            true
        }

        // Cache: if marker file matches current ISO's cache key, skip extraction.
        let marker = dest_dir.join(".knofoo_esp_cache_key");
        let want_key = Self::iso_cache_key(iso).unwrap_or_default();
        if let Ok(have) = std::fs::read_to_string(&marker) {
            if have == want_key {
                return true;
            }
        }

        // Read root directory
        let mut root_data = Vec::new();
        for c in read_fat_chain(&mut f, root_cluster) {
            if let Some(b) = read_cluster(&mut f, c) { root_data.extend_from_slice(&b); } else { return false; }
        }
        let entries = parse_dir(&root_data);

        let _ = std::fs::remove_dir_all(dest_dir);
        if std::fs::create_dir_all(dest_dir).is_err() { return false; }

        let read_chain_box = |f: &mut std::fs::File, start: u32| -> Vec<u32> {
            let mut chain = vec![start];
            loop {
                let last = *chain.last().unwrap();
                let fat_off = part_offset + reserved_secs as u64 * bytes_per_sec as u64 + last as u64 * 4;
                if f.seek(SeekFrom::Start(fat_off)).is_err() { break; }
                let mut b = [0u8; 4];
                if f.read_exact(&mut b).is_err() { break; }
                let nxt = u32::from_le_bytes(b) & 0x0FFF_FFFF;
                if nxt >= 0x0FFF_FFF8 || nxt == 0 || nxt == last { break; }
                chain.push(nxt);
                if chain.len() > 1_000_000 { break; }
            }
            chain
        };
        let read_clu_box = |f: &mut std::fs::File, cl: u32| -> Option<Vec<u8>> {
            let off = part_offset + (data_start_sec + (cl - 2) * sec_per_cluster) as u64 * bytes_per_sec as u64;
            f.seek(SeekFrom::Start(off)).ok()?;
            let mut buf = vec![0u8; cluster_size as usize];
            f.read_exact(&mut buf).ok()?;
            Some(buf)
        };

        let ok = copy_recursive(&mut f, entries, dest_dir, &read_chain_box, &read_clu_box);
        if !ok { return false; }

        // Also copy any files from the ISO9660 /EFI/ tree (in case bootloader
        // expects payloads not in the EFI FAT — e.g. kernel.bin).
        Self::copy_iso9660_efi_tree(iso, dest_dir);

        // Mark cache as valid for this ISO version.
        let _ = std::fs::write(&marker, &want_key);
        true
    }

    /// Walk the ISO9660 /EFI/ tree and copy any FILE entries into `dest_dir`,
    /// preserving paths. Skips files already present (so the FAT-extracted
    /// BOOTX64.EFI is not overwritten).
    fn copy_iso9660_efi_tree(iso: &std::path::Path, dest_dir: &std::path::Path) {
        use std::io::{Read, Seek, SeekFrom};
        let mut f = match std::fs::File::open(iso) { Ok(f) => f, Err(_) => return };

        // PVD at LBA 16
        if f.seek(SeekFrom::Start(16 * 2048)).is_err() { return; }
        let mut pvd = vec![0u8; 2048];
        if f.read_exact(&mut pvd).is_err() { return; }
        if &pvd[1..6] != b"CD001" { return; }

        // Root directory record at offset 156, 34 bytes
        let root_rec = &pvd[156..156 + 34];
        let root_lba = u32::from_le_bytes([root_rec[2], root_rec[3], root_rec[4], root_rec[5]]);
        let root_size = u32::from_le_bytes([root_rec[10], root_rec[11], root_rec[12], root_rec[13]]);

        // Find /EFI/ entry
        let efi_rec = match Self::iso9660_find_child(&mut f, root_lba, root_size, "EFI") {
            Some(r) => r, None => return,
        };
        if efi_rec.2 == 0 { return; }
        Self::iso9660_copy_dir_recursive(&mut f, efi_rec.1, efi_rec.2, &dest_dir.join("EFI"));
    }

    /// Returns (name, lba, size) of a child entry matching `target` (case-insensitive,
    /// strips ;version suffix), or None.
    fn iso9660_find_child(
        f: &mut std::fs::File,
        dir_lba: u32,
        dir_size: u32,
        target: &str,
    ) -> Option<(String, u32, u32)> {
        use std::io::{Read, Seek, SeekFrom};
        f.seek(SeekFrom::Start(dir_lba as u64 * 2048)).ok()?;
        let mut data = vec![0u8; dir_size as usize];
        f.read_exact(&mut data).ok()?;
        let target_up = target.to_ascii_uppercase();

        let mut i = 0usize;
        while i < data.len() {
            let rlen = data[i] as usize;
            if rlen == 0 { i += 1; continue; }
            if i + rlen > data.len() { break; }
            let rec = &data[i..i + rlen];
            let lba = u32::from_le_bytes([rec[2], rec[3], rec[4], rec[5]]);
            let sz = u32::from_le_bytes([rec[10], rec[11], rec[12], rec[13]]);
            let name_len = rec[32] as usize;
            let raw = &rec[33..33 + name_len];
            let name = String::from_utf8_lossy(raw).to_ascii_uppercase();
            // strip trailing ;version
            let bare = name.split(';').next().unwrap_or("");
            if bare == target_up {
                return Some((bare.to_string(), lba, sz));
            }
            i += rlen;
        }
        None
    }

    fn iso9660_copy_dir_recursive(
        f: &mut std::fs::File,
        dir_lba: u32,
        dir_size: u32,
        dest_dir: &std::path::Path,
    ) {
        use std::io::{Read, Seek, SeekFrom};
        if f.seek(SeekFrom::Start(dir_lba as u64 * 2048)).is_err() { return; }
        let mut data = vec![0u8; dir_size as usize];
        if f.read_exact(&mut data).is_err() { return; }
        let _ = std::fs::create_dir_all(dest_dir);

        let mut i = 0usize;
        while i < data.len() {
            let rlen = data[i] as usize;
            if rlen == 0 { i += 1; continue; }
            if i + rlen > data.len() { break; }
            let rec = &data[i..i + rlen];
            let lba = u32::from_le_bytes([rec[2], rec[3], rec[4], rec[5]]);
            let sz = u32::from_le_bytes([rec[10], rec[11], rec[12], rec[13]]);
            let flags = rec[25];
            let name_len = rec[32] as usize;
            let raw = &rec[33..33 + name_len];
            // Skip self/parent (single-byte 0x00/0x01)
            if name_len == 1 && (raw[0] == 0 || raw[0] == 1) { i += rlen; continue; }
            let name = String::from_utf8_lossy(raw).to_string();
            let bare = name.split(';').next().unwrap_or("").to_string();
            if bare.is_empty() { i += rlen; continue; }
            let is_dir = flags & 2 != 0;

            if is_dir {
                Self::iso9660_copy_dir_recursive(f, lba, sz, &dest_dir.join(&bare));
            } else {
                let out_path = dest_dir.join(&bare);
                if !out_path.exists() {
                    if let Ok(mut out) = std::fs::File::create(&out_path) {
                        // Stream the file in 1MB chunks
                        let mut remaining = sz as u64;
                        let mut off = lba as u64 * 2048;
                        let mut buf = vec![0u8; 1024 * 1024];
                        while remaining > 0 {
                            if f.seek(SeekFrom::Start(off)).is_err() { break; }
                            let chunk = remaining.min(buf.len() as u64) as usize;
                            if f.read_exact(&mut buf[..chunk]).is_err() { break; }
                            use std::io::Write;
                            if out.write_all(&buf[..chunk]).is_err() { break; }
                            off += chunk as u64;
                            remaining -= chunk as u64;
                        }
                    }
                }
            }
            i += rlen;
        }
    }

    fn build_args(&self, config: &MachineConfig) -> Vec<String> {
        let display_num = self.vnc_port - 5900;
        let mut args = vec![
            "-machine".into(), "q35".into(),
            "-cpu".into(), "max".into(),
            "-m".into(), config.ram_mb.to_string(),
            "-smp".into(), config.cpus.to_string(),
            "-display".into(), "none".into(),
            "-vnc".into(), format!("127.0.0.1:{}", display_num),
            "-vga".into(), "std".into(),
            "-no-reboot".into(),
        ];

        // Agent channel: a virtio-serial port bridged to the host over a TCP chardev
        // socket. QEMU listens (server=on,wait=off); the host agent-bridge thread
        // connects, mirroring the VNC client. Guest sees it as /dev/vport0p1
        // (name "foo.knofoo.agent" in /sys/class/virtio-ports).
        args.extend([
            "-device".into(), "virtio-serial-pci,id=vser0".into(),
            "-chardev".into(),
            format!("socket,id=agent0,host=127.0.0.1,port={},server=on,wait=off", self.agent_port),
            "-device".into(), "virtserialport,bus=vser0.0,chardev=agent0,name=foo.knofoo.agent".into(),
        ]);

        // Resolve effective boot mode — auto-detect when set to Auto.
        let effective_mode: BootMode = match &config.boot_mode {
            BootMode::Auto => {
                if let Some(path) = &config.iso_path {
                    let p = std::path::Path::new(path);
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                    if ext == "img" || ext == "qcow2" {
                        BootMode::Disk
                    } else if ext == "iso" {
                        match Self::detect_iso_boot_kind(p) {
                            IsoBootKind::UefiHybrid | IsoBootKind::UefiElTorito => BootMode::Uefi,
                            IsoBootKind::BiosOnly => BootMode::Bios,
                            IsoBootKind::Unknown => BootMode::Bios,
                        }
                    } else {
                        BootMode::Bios
                    }
                } else {
                    BootMode::Bios
                }
            }
            other => other.clone(),
        };
        eprintln!("[FallbackBackend] boot mode: {:?} (configured: {:?})", effective_mode, config.boot_mode);

        // Firmware setup
        if effective_mode == BootMode::Uefi {
            if let Some((code, vars_template)) = Self::ovmf_split() {
                if let Some(vars_path) = Self::vm_vars_path(&config.id) {
                    let _ = std::fs::copy(&vars_template, &vars_path);
                    args.extend([
                        "-drive".into(),
                        format!("if=pflash,format=raw,unit=0,readonly=on,file={}", code.to_string_lossy()),
                    ]);
                    args.extend([
                        "-drive".into(),
                        format!("if=pflash,format=raw,unit=1,file={}", vars_path.to_string_lossy()),
                    ]);
                } else if let Some(ovmf) = Self::ovmf_path() {
                    args.extend(["-bios".into(), ovmf.to_string_lossy().into_owned()]);
                }
            } else if let Some(ovmf) = Self::ovmf_path() {
                args.extend(["-bios".into(), ovmf.to_string_lossy().into_owned()]);
            } else {
                eprintln!("[FallbackBackend] WARNING: UEFI requested but no OVMF firmware found");
            }
        }
        // BIOS / Disk modes use QEMU's default SeaBIOS — no firmware flag needed.

        // Common devices.
        args.extend([
            "-device".into(), "qemu-xhci,id=xhci".into(),
            "-device".into(), "usb-tablet,bus=xhci.0".into(),
            "-device".into(), "virtio-rng-pci".into(),
        ]);

        // Boot media setup
        if let Some(path) = &config.iso_path {
            let p = std::path::Path::new(path);
            match effective_mode {
                BootMode::Uefi => {
                    // For UEFI hybrid ISOs: extract ESP + virtual FAT (works around
                    // OVMF's unreliable El Torito UEFI handling on q35). Falls back
                    // to plain -cdrom for ISOs whose ESP can't be extracted.
                    let mut esp_attached = false;
                    if let Some(esp_dir) = Self::vm_esp_dir(&config.id) {
                        if Self::extract_esp_from_iso(p, &esp_dir) {
                            eprintln!("[FallbackBackend] extracted ESP to {}", esp_dir.display());
                            args.extend([
                                "-drive".into(),
                                format!("format=raw,file=fat:rw:{}", esp_dir.to_string_lossy()),
                            ]);
                            esp_attached = true;
                        }
                    }
                    args.extend(["-cdrom".into(), path.clone()]);
                    if !esp_attached {
                        // No ESP available — try El Torito UEFI boot from cdrom directly.
                        args.extend(["-boot".into(), "d".into()]);
                    }
                }
                BootMode::Bios => {
                    args.extend(["-cdrom".into(), path.clone()]);
                    args.extend(["-boot".into(), "d".into()]);
                }
                BootMode::Disk => {
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                    let fmt = if ext == "qcow2" { "qcow2" } else { "raw" };
                    args.extend([
                        "-drive".into(),
                        format!("file={},format={},if=virtio,index=0,media=disk", path, fmt),
                    ]);
                }
                BootMode::Auto => unreachable!("auto resolved above"),
            }
        }

        if !config.disk_path.is_empty() && std::path::Path::new(&config.disk_path).exists() {
            args.extend(["-drive".into(), format!("file={},format=raw,if=virtio", config.disk_path)]);
        }

        if let Some(shared) = &config.shared_folder {
            args.extend([
                "-virtfs".into(),
                format!("local,path={},mount_tag=workspace,security_model=mapped-xattr,id=fsdev0", shared),
            ]);
        }

        if config.network.internet {
            let mut netdev = "user,id=net0".to_string();
            for pf in &config.network.port_forwards {
                netdev.push_str(&format!(",hostfwd={}::{}-:{}", pf.proto, pf.host, pf.guest));
            }
            args.extend(["-netdev".into(), netdev, "-device".into(), "virtio-net-pci,netdev=net0".into()]);
        } else {
            args.extend(["-nic".into(), "none".into()]);
        }

        // Hardware acceleration
        #[cfg(target_os = "linux")]
        if std::path::Path::new("/dev/kvm").exists() {
            args.extend(["-accel".into(), "kvm".into()]);
        }
        #[cfg(target_os = "macos")]
        args.extend(["-accel".into(), "hvf".into()]);
        #[cfg(target_os = "windows")]
        args.extend(["-accel".into(), "whpx,kernel-irqchip=off".into()]);

        args
    }

    fn start_vnc_client(
        port: u16,
        gpu: Arc<Mutex<GpuDevice>>,
        stop: Arc<std::sync::atomic::AtomicBool>,
        error_msg: Arc<Mutex<Option<String>>>,
        input_stream: Arc<Mutex<Option<TcpStream>>>,
        vnc_connected: Arc<std::sync::atomic::AtomicBool>,
    ) {
        std::thread::spawn(move || {
            eprintln!("[VNC] thread started, waiting 1.5s for QEMU...");
            std::thread::sleep(Duration::from_millis(1500));

            let addr = format!("127.0.0.1:{}", port);
            eprintln!("[VNC] connecting to {}", addr);
            let stream = match Self::connect_with_retry(&addr, 20) {
                Some(s) => { eprintln!("[VNC] connected"); s }
                None => {
                    let msg = format!("VNC: could not connect to {} after retries", addr);
                    eprintln!("[VNC] {}", msg);
                    *error_msg.lock().unwrap() = Some(msg);
                    return;
                }
            };

            if let Ok(writer) = stream.try_clone() {
                *input_stream.lock().unwrap() = Some(writer);
            }
            vnc_connected.store(true, Ordering::Relaxed);

            eprintln!("[VNC] starting RFB session");
            if let Err(e) = Self::rfb_session(stream, gpu, stop, input_stream) {
                eprintln!("[VNC] session error: {}", e);
                *error_msg.lock().unwrap() = Some(e);
            } else {
                eprintln!("[VNC] session ended cleanly");
            }
        });
    }

    fn connect_with_retry(addr: &str, attempts: u32) -> Option<TcpStream> {
        for i in 0..attempts {
            match TcpStream::connect(addr) {
                Ok(s) => return Some(s),
                Err(_) => std::thread::sleep(Duration::from_millis(500 + i as u64 * 200)),
            }
        }
        None
    }

    // Bridges the shared SerialDevice to the guest's virtio-serial port.
    // Poll loop (50ms read timeout): guest→host bytes go to SerialDevice::recv_from_guest,
    // host→guest bytes are drained from SerialDevice::next_tx and written to the socket.
    fn start_agent_bridge(
        port: u16,
        serial: Arc<Mutex<SerialDevice>>,
        agent_stream: Arc<Mutex<Option<TcpStream>>>,
        stop: Arc<std::sync::atomic::AtomicBool>,
    ) {
        std::thread::spawn(move || {
            eprintln!("[agent] thread started, waiting 1.5s for QEMU...");
            std::thread::sleep(Duration::from_millis(1500));

            let addr = format!("127.0.0.1:{}", port);
            eprintln!("[agent] connecting to {}", addr);
            let mut reader = match Self::connect_with_retry(&addr, 20) {
                Some(s) => { eprintln!("[agent] connected"); s }
                None => { eprintln!("[agent] could not connect to {}", addr); return; }
            };
            reader.set_read_timeout(Some(Duration::from_millis(50))).ok();
            if let Ok(writer) = reader.try_clone() {
                *agent_stream.lock().unwrap() = Some(writer);
            }

            let mut buf = [0u8; 8192];
            loop {
                if stop.load(Ordering::Relaxed) { break; }

                // guest → host
                match reader.read(&mut buf) {
                    Ok(0) => { eprintln!("[agent] socket closed by QEMU"); break; }
                    Ok(n) => {
                        if let Ok(mut s) = serial.lock() {
                            s.recv_from_guest(&buf[..n]);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock
                               || e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(e) => { eprintln!("[agent] read error: {}", e); break; }
                }

                // host → guest (drain queue without holding the serial lock during writes)
                let pending: Vec<Vec<u8>> = {
                    let mut out = Vec::new();
                    if let Ok(mut s) = serial.lock() {
                        while let Some(data) = s.next_tx() { out.push(data); }
                    }
                    out
                };
                for data in pending {
                    if let Some(w) = agent_stream.lock().unwrap().as_mut() {
                        if w.write_all(&data).is_err() {
                            eprintln!("[agent] write error");
                            break;
                        }
                    }
                }
            }

            *agent_stream.lock().unwrap() = None;
            eprintln!("[agent] bridge thread ended");
        });
    }

    fn rfb_session(
        mut stream: TcpStream,
        gpu: Arc<Mutex<GpuDevice>>,
        stop: Arc<std::sync::atomic::AtomicBool>,
        input_stream: Arc<Mutex<Option<TcpStream>>>,
    ) -> Result<(), String> {
        use std::sync::atomic::Ordering;

        // RFB handshake
        let mut ver = [0u8; 12];
        stream.read_exact(&mut ver).map_err(|e| e.to_string())?;
        stream.write_all(b"RFB 003.008\n").map_err(|e| e.to_string())?;

        // Security types
        let mut n_sec = [0u8; 1];
        stream.read_exact(&mut n_sec).map_err(|e| e.to_string())?;
        let mut sec_types = vec![0u8; n_sec[0] as usize];
        stream.read_exact(&mut sec_types).map_err(|e| e.to_string())?;
        let chosen = if sec_types.contains(&1) { 1u8 } else { sec_types[0] };
        stream.write_all(&[chosen]).map_err(|e| e.to_string())?;

        // RFB 3.8: SecurityResult is always sent (even for type 1 / None)
        {
            let mut result = [0u8; 4];
            stream.read_exact(&mut result).map_err(|e| e.to_string())?;
            if u32::from_be_bytes(result) != 0 {
                return Err("VNC security result: failed".into());
            }
        }

        // ClientInit: shared=1
        stream.write_all(&[1u8]).map_err(|e| e.to_string())?;

        // ServerInit
        let mut sinit = [0u8; 24];
        stream.read_exact(&mut sinit).map_err(|e| e.to_string())?;
        let fb_w = u16::from_be_bytes([sinit[0], sinit[1]]) as u32;
        let fb_h = u16::from_be_bytes([sinit[2], sinit[3]]) as u32;

        let name_len = u32::from_be_bytes([sinit[20], sinit[21], sinit[22], sinit[23]]) as usize;
        let mut name = vec![0u8; name_len];
        stream.read_exact(&mut name).map_err(|e| e.to_string())?;

        eprintln!("[VNC] ServerInit: {}x{}", fb_w, fb_h);
        gpu.lock().unwrap().resize(fb_w, fb_h);

        if let Ok(writer) = stream.try_clone() {
            *input_stream.lock().unwrap() = Some(writer);
        }

        // SetEncodings: Raw only
        stream.write_all(&[
            2, 0,    // SetEncodings, padding
            0, 1,    // count = 1
            0, 0, 0, 0, // Raw = 0
        ]).map_err(|e| e.to_string())?;

        // SetPixelFormat: 32bpp BGRA
        stream.write_all(&[
            0,          // SetPixelFormat
            0, 0, 0,    // padding
            32,         // bits-per-pixel
            24,         // depth
            0,          // big-endian
            1,          // true-color
            0, 255,     // red-max
            0, 255,     // green-max
            0, 255,     // blue-max
            16,         // red-shift
            8,          // green-shift
            0,          // blue-shift
            0, 0, 0,    // padding
        ]).map_err(|e| e.to_string())?;

        // Reader is fully blocking — no timeout. A separate writer thread sends
        // incremental requests on a timer so the reader never stalls mid-frame.
        stream.set_read_timeout(None).ok();

        // Writer thread: send non-incremental first, then incremental every 33ms
        let mut writer = stream.try_clone().map_err(|e| e.to_string())?;
        let stop_w = Arc::clone(&stop);
        std::thread::spawn(move || {
            let full_req: Vec<u8> = vec![
                3, 0, 0, 0, 0, 0,
                (fb_w >> 8) as u8, fb_w as u8,
                (fb_h >> 8) as u8, fb_h as u8,
            ];
            let incr_req: Vec<u8> = vec![
                3, 1, 0, 0, 0, 0,
                (fb_w >> 8) as u8, fb_w as u8,
                (fb_h >> 8) as u8, fb_h as u8,
            ];
            eprintln!("[VNC writer] sending full_req");
            if let Err(e) = writer.write_all(&full_req) {
                eprintln!("[VNC writer] full_req failed: {}", e);
                return;
            }
            let mut tick = 0u32;
            loop {
                std::thread::sleep(Duration::from_millis(33));
                if stop_w.load(Ordering::Relaxed) { eprintln!("[VNC writer] stop"); break; }
                tick += 1;
                if let Err(e) = writer.write_all(&incr_req) {
                    eprintln!("[VNC writer] incr_req failed at tick {}: {}", tick, e);
                    break;
                }
                if tick % 30 == 1 { eprintln!("[VNC writer] tick {}", tick); }
            }
            eprintln!("[VNC writer] thread exiting");
        });

        eprintln!("[VNC] entering frame loop (blocking reader)");
        let mut frames_received = 0u32;

        loop {
            if stop.load(Ordering::Relaxed) { break; }

            let mut msg_type = [0u8; 1];
            if stream.read_exact(&mut msg_type).is_err() { break; }

            match msg_type[0] {
                0 => {
                    let mut hdr = [0u8; 3];
                    if stream.read_exact(&mut hdr).is_err() { break; }
                    let n_rects = u16::from_be_bytes([hdr[1], hdr[2]]);

                    for _ in 0..n_rects {
                        let mut rhdr = [0u8; 12];
                        if stream.read_exact(&mut rhdr).is_err() { break; }
                        let rx  = u16::from_be_bytes([rhdr[0], rhdr[1]]) as u32;
                        let ry  = u16::from_be_bytes([rhdr[2], rhdr[3]]) as u32;
                        let rw  = u16::from_be_bytes([rhdr[4], rhdr[5]]) as u32;
                        let rh  = u16::from_be_bytes([rhdr[6], rhdr[7]]) as u32;
                        let enc = i32::from_be_bytes([rhdr[8], rhdr[9], rhdr[10], rhdr[11]]);

                        if enc == 0 {
                            let pixel_bytes = (rw * rh * 4) as usize;
                            let mut pixels = vec![0u8; pixel_bytes];
                            if stream.read_exact(&mut pixels).is_err() { break; }

                            // BGRX → RGBA
                            for px in pixels.chunks_exact_mut(4) {
                                let b = px[0]; let g = px[1]; let r = px[2];
                                px[0] = r; px[1] = g; px[2] = b; px[3] = 255;
                            }

                            gpu.lock().unwrap().update_region(rx, ry, rw, rh, &pixels);
                            frames_received += 1;
                            if frames_received % 30 == 1 {
                                eprintln!("[VNC] frame {}: rect {},{}  {}x{}", frames_received, rx, ry, rw, rh);
                            }
                        } else {
                            let skip = (rw * rh * 4) as usize;
                            let mut buf = vec![0u8; skip];
                            stream.read_exact(&mut buf).ok();
                        }
                    }
                }
                2 => {
                    // SetColourMapEntries — skip
                    let mut hdr = [0u8; 5];
                    stream.read_exact(&mut hdr).ok();
                    let n = u16::from_be_bytes([hdr[3], hdr[4]]) as usize;
                    let mut skip = vec![0u8; n * 6];
                    stream.read_exact(&mut skip).ok();
                }
                3 => {} // Bell
                4 => {
                    // ServerCutText
                    let mut hdr = [0u8; 7];
                    stream.read_exact(&mut hdr).ok();
                    let len = u32::from_be_bytes([hdr[3], hdr[4], hdr[5], hdr[6]]) as usize;
                    let mut text = vec![0u8; len];
                    stream.read_exact(&mut text).ok();
                }
                _ => { eprintln!("[VNC] unknown msg type {}", msg_type[0]); }
            }
        }

        *input_stream.lock().unwrap() = None;
        Ok(())
    }

    fn send_rfb_key(stream: &Arc<Mutex<Option<TcpStream>>>, keycode: u32, down: bool) {
        if let Some(ref mut s) = *stream.lock().unwrap() {
            // RFB KeyEvent: type=4, down-flag, padding, key-sym (X11 keysym)
            let msg = [
                4u8,
                down as u8,
                0, 0,
                (keycode >> 24) as u8,
                (keycode >> 16) as u8,
                (keycode >> 8) as u8,
                keycode as u8,
            ];
            let _ = s.write_all(&msg);
        }
    }

    fn send_rfb_mouse(stream: &Arc<Mutex<Option<TcpStream>>>, x: i32, y: i32, buttons: u8) {
        if let Some(ref mut s) = *stream.lock().unwrap() {
            let x = x.max(0) as u16;
            let y = y.max(0) as u16;
            // RFB PointerEvent: type=5, button-mask, x-pos, y-pos
            let msg = [
                5u8,
                buttons,
                (x >> 8) as u8, x as u8,
                (y >> 8) as u8, y as u8,
            ];
            let _ = s.write_all(&msg);
        }
    }
}

fn qemu_binary_name() -> &'static str {
    #[cfg(target_os = "windows")] return "qemu-system-x86_64.exe";
    #[cfg(not(target_os = "windows"))] return "qemu-system-x86_64";
}

impl HvBackend for FallbackBackend {
    fn start(&mut self, config: &MachineConfig) -> Result<(), String> {
        eprintln!("[FallbackBackend::start] called");
        let qemu = Self::qemu_binary()
            .ok_or("QEMU not found. Install qemu-system-x86 and retry.")?;
        eprintln!("[FallbackBackend::start] qemu binary: {:?}", qemu);

        let args = self.build_args(config);
        eprintln!("[FallbackBackend::start] args: {:?}", args);

        eprintln!("[FallbackBackend::start] spawning QEMU...");
        let mut child = Command::new(&qemu)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("QEMU spawn: {}", e))?;
        eprintln!("[FallbackBackend::start] QEMU spawned, pid={}", child.id());

        // Give QEMU 2s to fail fast (bad args, missing file, etc.)
        eprintln!("[FallbackBackend::start] waiting 2s to detect early exit...");
        std::thread::sleep(std::time::Duration::from_millis(2000));
        eprintln!("[FallbackBackend::start] checking exit status...");
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stderr_out = String::new();
                if let Some(mut e) = child.stderr.take() {
                    use std::io::Read;
                    e.read_to_string(&mut stderr_out).ok();
                }
                return Err(format!("QEMU exited immediately ({}): {}", status, stderr_out.trim()));
            }
            _ => {}
        }

        eprintln!("[FallbackBackend::start] QEMU still alive, starting VNC client...");
        self.child = Some(child);
        self.stop_flag.store(false, Ordering::Relaxed);
        self.vnc_connected.store(false, Ordering::Relaxed);
        *self.input_stream.lock().unwrap() = None;
        *self.agent_stream.lock().unwrap() = None;
        *self.error_msg.lock().unwrap() = None;

        let gpu    = Arc::clone(&self.gpu);
        let stop   = Arc::clone(&self.stop_flag);
        let error  = Arc::clone(&self.error_msg);
        let input  = Arc::clone(&self.input_stream);
        let port   = self.vnc_port;
        let connected = Arc::clone(&self.vnc_connected);
        Self::start_vnc_client(port, gpu, stop, error, input, connected);

        // Agent channel bridge: connects to QEMU's virtio-serial socket and shuttles
        // bytes between the shared SerialDevice and the guest.
        Self::start_agent_bridge(
            self.agent_port,
            Arc::clone(&self.serial),
            Arc::clone(&self.agent_stream),
            Arc::clone(&self.stop_flag),
        );

        self.state = VmState::Starting;
        eprintln!("[FallbackBackend::start] returning Ok, state=Starting");
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
        self.vnc_connected.store(false, std::sync::atomic::Ordering::Relaxed);
        *self.input_stream.lock().unwrap() = None;
        *self.agent_stream.lock().unwrap() = None;
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
        self.state = VmState::Stopped;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), String> { self.state = VmState::Paused;   Ok(()) }
    fn resume(&mut self) -> Result<(), String> { self.state = VmState::Running; Ok(()) }

    fn send_key(&mut self, keycode: u32, pressed: bool) -> Result<(), String> {
        Self::send_rfb_key(&self.input_stream, keycode, pressed);
        Ok(())
    }

    fn send_mouse(&mut self, x: i32, y: i32, cmd: u8) -> Result<(), String> {
        // cmd encoding (set by machine_manager::send_input):
        //   0x00       → pointer move only, use current button_mask
        //   0x80 | bit → press: set bit in mask, then send
        //   0x40 | bit → release: clear bit in mask, then send
        //   0x08       → wheel one-shot (x=dx, y=dy in scroll deltas)
        let mask = if cmd == 0 {
            self.button_mask
        } else if cmd & 0x80 != 0 {
            self.button_mask |= cmd & 0x07;
            self.button_mask
        } else if cmd & 0x40 != 0 {
            self.button_mask &= !(cmd & 0x07);
            self.button_mask
        } else if cmd & 0x08 != 0 {
            // Translate wheel delta into RFB scroll button presses.
            // RFB has no native wheel — emit press+release of buttons 4 (up) / 5 (down).
            // dy < 0 → wheel up, dy > 0 → wheel down.
            let dy = y;
            if dy != 0 {
                let bit = if dy < 0 { 1u8 << 3 } else { 1u8 << 4 };
                let last_x = 0; let last_y = 0; // RFB scroll uses cursor pos; 0,0 ok
                Self::send_rfb_mouse(&self.input_stream, last_x, last_y, self.button_mask | bit);
                Self::send_rfb_mouse(&self.input_stream, last_x, last_y, self.button_mask);
            }
            return Ok(());
        } else {
            self.button_mask
        };
        Self::send_rfb_mouse(&self.input_stream, x, y, mask);
        Ok(())
    }

    fn get_frame(&mut self) -> Option<super::FrameSnapshot> {
        self.gpu.lock().unwrap().take_snapshot()
    }

    fn get_frame_bin(&mut self) -> Vec<u8> {
        self.gpu.lock().unwrap().take_snapshot_bin()
    }

    fn send_serial(&mut self, _: &[u8]) -> Result<(), String> { Ok(()) }
    fn recv_serial(&mut self) -> Option<Vec<u8>> { None }

    fn get_dirty_rects(&mut self) -> Vec<FrameRect> {
        let rects = self.gpu.lock().unwrap().take_dirty_rects();
        if !rects.is_empty() {
            eprintln!("[GPU] emitting {} dirty rects, state={:?}", rects.len(), self.state);
            if self.state == VmState::Starting {
                eprintln!("[GPU] first frame → Running");
                self.state = VmState::Running;
            }
        }
        rects
    }

    fn state(&mut self) -> VmState {
        if self.state == VmState::Starting || self.state == VmState::Running {
            if self.error_msg.lock().unwrap().is_some() {
                self.state = VmState::Error;
            } else if self.state == VmState::Starting
                && self.vnc_connected.load(Ordering::Relaxed)
            {
                // VNC session established → show display (frames may still be coming)
                self.state = VmState::Running;
            }
        }
        self.state.clone()
    }

    fn framebuffer_size(&self) -> (u32, u32) {
        let g = self.gpu.lock().unwrap();
        (g.width, g.height)
    }
}

impl Drop for FallbackBackend {
    fn drop(&mut self) {
        self.stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
        *self.input_stream.lock().unwrap() = None;
        *self.agent_stream.lock().unwrap() = None;
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}
