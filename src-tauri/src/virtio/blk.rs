// virtio-blk: disk device emulation
// Handles read/write requests from guest OS to ISO/qcow2 disk images

use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;

pub struct BlkDevice {
    file: File,
    read_only: bool,
    sector_size: u64,
    num_sectors: u64,
}

impl BlkDevice {
    pub fn open_iso(path: &str) -> Result<Self, String> {
        let f = File::open(path).map_err(|e| format!("open iso {}: {}", path, e))?;
        let meta = f.metadata().map_err(|e| e.to_string())?;
        let size = meta.len();
        Ok(Self {
            file: f,
            read_only: true,
            sector_size: 512,
            num_sectors: size / 512,
        })
    }

    pub fn open_or_create_qcow2(path: &str, size_mb: u64) -> Result<Self, String> {
        if !Path::new(path).exists() {
            create_raw_disk(path, size_mb)?;
        }
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| format!("open disk {}: {}", path, e))?;
        let meta = f.metadata().map_err(|e| e.to_string())?;
        let size = meta.len();
        Ok(Self {
            file: f,
            read_only: false,
            sector_size: 512,
            num_sectors: size / 512,
        })
    }

    pub fn read_sectors(&mut self, sector: u64, count: u64) -> Result<Vec<u8>, String> {
        let offset = sector * self.sector_size;
        let len = (count * self.sector_size) as usize;
        self.file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; len];
        self.file.read_exact(&mut buf).map_err(|e| e.to_string())?;
        Ok(buf)
    }

    pub fn write_sectors(&mut self, sector: u64, data: &[u8]) -> Result<(), String> {
        if self.read_only {
            return Err("disk is read-only".into());
        }
        let offset = sector * self.sector_size;
        self.file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
        self.file.write_all(data).map_err(|e| e.to_string())
    }

    pub fn sector_count(&self) -> u64 { self.num_sectors }
    pub fn sector_size(&self) -> u64 { self.sector_size }
}

fn create_raw_disk(path: &str, size_mb: u64) -> Result<(), String> {
    let size = size_mb * 1024 * 1024;
    let f = File::create(path).map_err(|e| e.to_string())?;
    f.set_len(size).map_err(|e| e.to_string())?;
    Ok(())
}
