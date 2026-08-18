// virtio-9p: shared folder device (pure Rust 9P2000.L server)
// Guest mounts: mount -t 9p -o trans=virtio workspace /workspace
// Host path: shared_folder from MachineConfig

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;

pub struct P9Device {
    root: PathBuf,
    fids: HashMap<u32, P9Fid>,
    next_qid: u64,
}

#[derive(Clone)]
struct P9Fid {
    path: PathBuf,
    open: bool,
}

// 9P2000.L message types
#[repr(u8)]
enum MsgType {
    Tversion = 100, Rversion = 101,
    Tattach  = 104, Rattach  = 105,
    Twalk    = 110, Rwalk    = 111,
    Topen    = 112, Ropen    = 113,
    Tread    = 116, Rread    = 117,
    Twrite   = 118, Rwrite   = 119,
    Tclunk   = 120, Rclunk   = 121,
    Tstat    = 124, Rstat    = 125,
    Tremove  = 122, Rremove  = 123,
    Tmkdir   = 72,  Rmkdir   = 73,
    Tunlinkat = 76, Runlinkat = 77,
}

impl P9Device {
    pub fn new(root: &str) -> Result<Self, String> {
        let root = PathBuf::from(root);
        if !root.exists() {
            fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        }
        Ok(Self { root, fids: HashMap::new(), next_qid: 1 })
    }

    pub fn handle(&mut self, msg: &[u8]) -> Vec<u8> {
        if msg.len() < 7 { return self.error(0, "short message"); }

        let _size   = u32::from_le_bytes(msg[0..4].try_into().unwrap_or([0;4]));
        let msg_type = msg[4];
        let tag     = u16::from_le_bytes(msg[5..7].try_into().unwrap_or([0;2]));
        let body    = &msg[7..];

        match msg_type {
            100 => self.handle_version(tag, body),   // Tversion
            104 => self.handle_attach(tag, body),    // Tattach
            110 => self.handle_walk(tag, body),      // Twalk
            112 => self.handle_open(tag, body),      // Topen
            116 => self.handle_read(tag, body),      // Tread
            118 => self.handle_write(tag, body),     // Twrite
            120 => self.handle_clunk(tag, body),     // Tclunk
            124 => self.handle_stat(tag, body),      // Tstat
            122 => self.handle_remove(tag, body),    // Tremove
            72  => self.handle_mkdir(tag, body),     // Tmkdir
            _   => self.error(tag, "unknown message"),
        }
    }

    fn handle_version(&self, tag: u16, body: &[u8]) -> Vec<u8> {
        let msize = 65536u32;
        let version = b"9P2000.L";
        let mut r = self.reply_header(101, tag, 4 + 2 + version.len());
        r.extend_from_slice(&msize.to_le_bytes());
        r.extend_from_slice(&(version.len() as u16).to_le_bytes());
        r.extend_from_slice(version);
        r
    }

    fn handle_attach(&mut self, tag: u16, body: &[u8]) -> Vec<u8> {
        if body.len() < 4 { return self.error(tag, "short attach"); }
        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        self.fids.insert(fid, P9Fid { path: self.root.clone(), open: false });
        let qid = self.make_qid(&self.root.clone());
        let mut r = self.reply_header(105, tag, 13);
        r.extend_from_slice(&qid);
        r
    }

    fn handle_walk(&mut self, tag: u16, body: &[u8]) -> Vec<u8> {
        if body.len() < 10 { return self.error(tag, "short walk"); }
        let fid     = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let newfid  = u32::from_le_bytes(body[4..8].try_into().unwrap());
        let nwname  = u16::from_le_bytes(body[8..10].try_into().unwrap());

        let base = match self.fids.get(&fid) {
            Some(f) => f.path.clone(),
            None    => return self.error(tag, "unknown fid"),
        };

        let mut path = base;
        let mut qids: Vec<[u8; 13]> = Vec::new();
        let mut off = 10usize;

        for _ in 0..nwname {
            if off + 2 > body.len() { break; }
            let name_len = u16::from_le_bytes(body[off..off+2].try_into().unwrap()) as usize;
            off += 2;
            if off + name_len > body.len() { break; }
            let name = std::str::from_utf8(&body[off..off+name_len]).unwrap_or("?");
            off += name_len;
            path = path.join(name);
            qids.push(self.make_qid(&path));
        }

        self.fids.insert(newfid, P9Fid { path, open: false });

        let mut r = self.reply_header(111, tag, 2 + qids.len() * 13);
        r.extend_from_slice(&(qids.len() as u16).to_le_bytes());
        for q in &qids { r.extend_from_slice(q); }
        r
    }

    fn handle_open(&mut self, tag: u16, body: &[u8]) -> Vec<u8> {
        if body.len() < 8 { return self.error(tag, "short open"); }
        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        if let Some(f) = self.fids.get_mut(&fid) {
            f.open = true;
            let path = f.path.clone();
            let qid = self.make_qid(&path);
            let mut r = self.reply_header(113, tag, 13 + 4);
            r.extend_from_slice(&qid);
            r.extend_from_slice(&0u32.to_le_bytes()); // iounit
            return r;
        }
        self.error(tag, "unknown fid")
    }

    fn handle_read(&self, tag: u16, body: &[u8]) -> Vec<u8> {
        if body.len() < 16 { return self.error(tag, "short read"); }
        let fid    = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let offset = u64::from_le_bytes(body[4..12].try_into().unwrap());
        let count  = u32::from_le_bytes(body[12..16].try_into().unwrap()) as usize;

        let fid_info = match self.fids.get(&fid) {
            Some(f) => f.clone(),
            None    => return self.error(tag, "unknown fid"),
        };

        if fid_info.path.is_dir() {
            let data = self.read_dir(&fid_info.path, offset, count);
            let mut r = self.reply_header(117, tag, 4 + data.len());
            r.extend_from_slice(&(data.len() as u32).to_le_bytes());
            r.extend_from_slice(&data);
            return r;
        }

        match fs::read(&fid_info.path) {
            Ok(contents) => {
                let start = offset as usize;
                let end = (start + count).min(contents.len());
                let slice = if start < contents.len() { &contents[start..end] } else { &[] };
                let mut r = self.reply_header(117, tag, 4 + slice.len());
                r.extend_from_slice(&(slice.len() as u32).to_le_bytes());
                r.extend_from_slice(slice);
                r
            }
            Err(e) => self.error(tag, &e.to_string()),
        }
    }

    fn handle_write(&self, tag: u16, body: &[u8]) -> Vec<u8> {
        if body.len() < 16 { return self.error(tag, "short write"); }
        let fid    = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let offset = u64::from_le_bytes(body[4..12].try_into().unwrap());
        let count  = u32::from_le_bytes(body[12..16].try_into().unwrap()) as usize;
        let data   = &body[16..16 + count.min(body.len().saturating_sub(16))];

        let path = match self.fids.get(&fid) {
            Some(f) => f.path.clone(),
            None    => return self.error(tag, "unknown fid"),
        };

        use std::io::{Write, Seek, SeekFrom};
        match fs::OpenOptions::new().write(true).create(true).open(&path) {
            Ok(mut f) => {
                if let Err(e) = f.seek(SeekFrom::Start(offset)) {
                    return self.error(tag, &e.to_string());
                }
                match f.write_all(data) {
                    Ok(_) => {
                        let mut r = self.reply_header(119, tag, 4);
                        r.extend_from_slice(&(data.len() as u32).to_le_bytes());
                        r
                    }
                    Err(e) => self.error(tag, &e.to_string()),
                }
            }
            Err(e) => self.error(tag, &e.to_string()),
        }
    }

    fn handle_clunk(&mut self, tag: u16, body: &[u8]) -> Vec<u8> {
        if body.len() < 4 { return self.error(tag, "short clunk"); }
        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        self.fids.remove(&fid);
        self.reply_header(121, tag, 0)
    }

    fn handle_stat(&self, tag: u16, body: &[u8]) -> Vec<u8> {
        if body.len() < 4 { return self.error(tag, "short stat"); }
        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let path = match self.fids.get(&fid) {
            Some(f) => f.path.clone(),
            None    => return self.error(tag, "unknown fid"),
        };
        let stat = self.make_stat(&path);
        let mut r = self.reply_header(125, tag, 2 + stat.len());
        r.extend_from_slice(&(stat.len() as u16).to_le_bytes());
        r.extend_from_slice(&stat);
        r
    }

    fn handle_remove(&mut self, tag: u16, body: &[u8]) -> Vec<u8> {
        if body.len() < 4 { return self.error(tag, "short remove"); }
        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let path = match self.fids.remove(&fid) {
            Some(f) => f.path,
            None    => return self.error(tag, "unknown fid"),
        };
        let result = if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        match result {
            Ok(_) => self.reply_header(123, tag, 0),
            Err(e) => self.error(tag, &e.to_string()),
        }
    }

    fn handle_mkdir(&self, tag: u16, body: &[u8]) -> Vec<u8> {
        if body.len() < 8 { return self.error(tag, "short mkdir"); }
        let dfid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let name_len = u16::from_le_bytes(body[4..6].try_into().unwrap()) as usize;
        if body.len() < 6 + name_len { return self.error(tag, "short mkdir name"); }
        let name = std::str::from_utf8(&body[6..6+name_len]).unwrap_or("?");

        let parent = match self.fids.get(&dfid) {
            Some(f) => f.path.clone(),
            None    => return self.error(tag, "unknown fid"),
        };
        let new_dir = parent.join(name);
        match fs::create_dir_all(&new_dir) {
            Ok(_) => {
                let qid = self.make_qid(&new_dir);
                let mut r = self.reply_header(73, tag, 13);
                r.extend_from_slice(&qid);
                r
            }
            Err(e) => self.error(tag, &e.to_string()),
        }
    }

    fn make_qid(&self, path: &Path) -> [u8; 13] {
        let is_dir = path.is_dir();
        let qtype: u8 = if is_dir { 0x80 } else { 0x00 };
        let version = 0u32;
        let path_hash = path_to_u64(path);
        let mut q = [0u8; 13];
        q[0] = qtype;
        q[1..5].copy_from_slice(&version.to_le_bytes());
        q[5..13].copy_from_slice(&path_hash.to_le_bytes());
        q
    }

    fn make_stat(&self, path: &Path) -> Vec<u8> {
        let name = path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let size = if path.is_file() {
            fs::metadata(path).map(|m| m.len()).unwrap_or(0)
        } else { 0 };
        let mode: u32 = if path.is_dir() { 0x80000000 | 0o755 } else { 0o644 };

        let mut s = Vec::new();
        s.extend_from_slice(&0u16.to_le_bytes()); // type
        s.extend_from_slice(&0u32.to_le_bytes()); // dev
        s.extend_from_slice(&self.make_qid(path)); // qid
        s.extend_from_slice(&mode.to_le_bytes());
        s.extend_from_slice(&0u32.to_le_bytes()); // atime
        s.extend_from_slice(&0u32.to_le_bytes()); // mtime
        s.extend_from_slice(&size.to_le_bytes());
        push_str(&mut s, &name);
        push_str(&mut s, "root");  // uid
        push_str(&mut s, "root");  // gid
        push_str(&mut s, "root");  // muid
        s
    }

    fn read_dir(&self, path: &Path, offset: u64, max: usize) -> Vec<u8> {
        let mut entries: Vec<_> = fs::read_dir(path)
            .into_iter().flatten().flatten().collect();
        entries.sort_by_key(|e| e.file_name());

        let mut out = Vec::new();
        for (i, entry) in entries.iter().enumerate() {
            if (i as u64) < offset { continue; }
            if out.len() >= max { break; }
            let stat = self.make_stat(&entry.path());
            let entry_size = (stat.len() as u16).to_le_bytes();
            out.extend_from_slice(&entry_size);
            out.extend_from_slice(&stat);
        }
        out
    }

    fn reply_header(&self, msg_type: u8, tag: u16, body_len: usize) -> Vec<u8> {
        let total = (7 + body_len) as u32;
        let mut r = Vec::with_capacity(7 + body_len);
        r.extend_from_slice(&total.to_le_bytes());
        r.push(msg_type);
        r.extend_from_slice(&tag.to_le_bytes());
        r
    }

    fn error(&self, tag: u16, msg: &str) -> Vec<u8> {
        let msg_bytes = msg.as_bytes();
        let mut r = self.reply_header(107, tag, 2 + msg_bytes.len());
        r.extend_from_slice(&(msg_bytes.len() as u16).to_le_bytes());
        r.extend_from_slice(msg_bytes);
        r
    }
}

fn push_str(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(&(s.len() as u16).to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
}

fn path_to_u64(path: &Path) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    h.finish()
}
