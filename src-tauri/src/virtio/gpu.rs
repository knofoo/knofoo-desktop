// virtio-gpu: framebuffer device
// Tracks dirty regions and emits minimal diffs for efficient IPC transfer

use base64::Engine as _;
use crate::hypervisor::{FrameRect, FrameSnapshot};

const DEFAULT_W: u32 = 1024;
const DEFAULT_H: u32 = 768;
const BYTES_PER_PIXEL: u32 = 4; // RGBA

pub struct GpuDevice {
    pub width: u32,
    pub height: u32,
    pub framebuffer: Vec<u8>,
    prev_framebuffer: Vec<u8>,
    dirty: bool,
    force_full: bool, // emit full frame on next take (after resize)
}

impl GpuDevice {
    pub fn new() -> Self {
        let size = (DEFAULT_W * DEFAULT_H * BYTES_PER_PIXEL) as usize;
        Self {
            width: DEFAULT_W,
            height: DEFAULT_H,
            framebuffer: vec![0u8; size],
            prev_framebuffer: vec![0u8; size],
            dirty: false,
            force_full: false,
        }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.width = w;
        self.height = h;
        let size = (w * h * BYTES_PER_PIXEL) as usize;
        self.framebuffer.resize(size, 0);
        // Set prev to all 0xFF so any real frame will be "dirty"
        self.prev_framebuffer = vec![0xFFu8; size];
        self.dirty = true;
        self.force_full = false;
    }

    pub fn update_region(&mut self, x: u32, y: u32, w: u32, h: u32, data: &[u8]) {
        let stride = self.width * BYTES_PER_PIXEL;
        for row in 0..h {
            let src_off = (row * w * BYTES_PER_PIXEL) as usize;
            let dst_off = ((y + row) * stride + x * BYTES_PER_PIXEL) as usize;
            let len = (w * BYTES_PER_PIXEL) as usize;
            if dst_off + len <= self.framebuffer.len() && src_off + len <= data.len() {
                self.framebuffer[dst_off..dst_off + len]
                    .copy_from_slice(&data[src_off..src_off + len]);
            }
        }
        self.dirty = true;
    }

    // Compute dirty rectangles by comparing current vs previous framebuffer.
    // Groups consecutive dirty rows into rects to minimize IPC payload.
    pub fn take_dirty_rects(&mut self) -> Vec<FrameRect> {
        if !self.dirty { return vec![]; }

        let stride = (self.width * BYTES_PER_PIXEL) as usize;
        let mut rects = Vec::new();
        let mut dirty_start: Option<u32> = None;

        for row in 0..self.height {
            let off = (row as usize) * stride;
            let end = off + stride;
            let row_dirty = self.framebuffer[off..end] != self.prev_framebuffer[off..end];

            match (row_dirty, dirty_start) {
                (true, None) => dirty_start = Some(row),
                (false, Some(start)) => {
                    let h = row - start;
                    let data_off = (start as usize) * stride;
                    let data_end = (row as usize) * stride;
                    rects.push(FrameRect {
                        x: 0,
                        y: start,
                        w: self.width,
                        h,
                        data: base64::engine::general_purpose::STANDARD
                            .encode(&self.framebuffer[data_off..data_end]),
                    });
                    dirty_start = None;
                }
                _ => {}
            }
        }

        // Flush remaining dirty rows
        if let Some(start) = dirty_start {
            let h = self.height - start;
            let data_off = (start as usize) * stride;
            rects.push(FrameRect {
                x: 0,
                y: start,
                w: self.width,
                h,
                data: base64::engine::general_purpose::STANDARD
                    .encode(&self.framebuffer[data_off..]),
            });
        }

        self.prev_framebuffer.copy_from_slice(&self.framebuffer);
        self.dirty = false;
        rects
    }

    // Returns full framebuffer snapshot if dirty since last call, else None.
    pub fn take_snapshot(&mut self) -> Option<FrameSnapshot> {
        if !self.dirty { return None; }
        self.prev_framebuffer.copy_from_slice(&self.framebuffer);
        self.dirty = false;
        Some(FrameSnapshot {
            w: self.width,
            h: self.height,
            data: base64::engine::general_purpose::STANDARD.encode(&self.framebuffer),
        })
    }

    // Binary snapshot: [w:u32 LE | h:u32 LE | rgba ...]. Skips base64 entirely.
    // Returns empty Vec when no new frame so callers can cheaply check len == 0.
    pub fn take_snapshot_bin(&mut self) -> Vec<u8> {
        if !self.dirty { return Vec::new(); }
        self.prev_framebuffer.copy_from_slice(&self.framebuffer);
        self.dirty = false;
        let mut out = Vec::with_capacity(8 + self.framebuffer.len());
        out.extend_from_slice(&self.width.to_le_bytes());
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&self.framebuffer);
        out
    }
}
