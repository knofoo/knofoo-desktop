// virtio-serial: bidirectional serial channel
// Used for: clipboard sync, graph node I/O ports (stdin/stdout/exit code)

use std::collections::VecDeque;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SerialOutput {
    pub data: String,
    pub port: SerialPort,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SerialPort {
    Stdout,    // graph node output port
    Clipboard, // clipboard sync channel
    Control,   // internal control messages
}

pub struct SerialDevice {
    tx_queue: VecDeque<Vec<u8>>, // host → guest
    rx_stdout: VecDeque<u8>,     // guest → host (node output)
    rx_clipboard: VecDeque<u8>,  // guest → host (clipboard)
    exit_code: Option<i32>,
}

impl SerialDevice {
    pub fn new() -> Self {
        Self {
            tx_queue: VecDeque::new(),
            rx_stdout: VecDeque::new(),
            rx_clipboard: VecDeque::new(),
            exit_code: None,
        }
    }

    pub fn send_to_guest(&mut self, data: &[u8]) {
        self.tx_queue.push_back(data.to_vec());
    }

    pub fn send_clipboard_to_guest(&mut self, text: &str) {
        // Protocol: prefix "CLIP:" so guest can route it
        let msg = format!("CLIP:{}\n", text);
        self.tx_queue.push_back(msg.into_bytes());
    }

    pub fn recv_from_guest(&mut self, data: &[u8]) {
        // Guest writes to serial; detect clipboard vs stdout by prefix
        if data.starts_with(b"CLIP:") {
            self.rx_clipboard.extend(&data[5..]);
        } else if data.starts_with(b"EXIT:") {
            if let Ok(s) = std::str::from_utf8(&data[5..]) {
                self.exit_code = s.trim().parse().ok();
            }
        } else {
            self.rx_stdout.extend(data);
        }
    }

    pub fn drain_stdout(&mut self) -> Option<String> {
        if self.rx_stdout.is_empty() { return None; }
        let bytes: Vec<u8> = self.rx_stdout.drain(..).collect();
        String::from_utf8_lossy(&bytes).into_owned().into()
    }

    pub fn drain_clipboard(&mut self) -> Option<String> {
        if self.rx_clipboard.is_empty() { return None; }
        let bytes: Vec<u8> = self.rx_clipboard.drain(..).collect();
        String::from_utf8_lossy(&bytes).into_owned().into()
    }

    pub fn take_exit_code(&mut self) -> Option<i32> {
        self.exit_code.take()
    }

    pub fn next_tx(&mut self) -> Option<Vec<u8>> {
        self.tx_queue.pop_front()
    }
}
