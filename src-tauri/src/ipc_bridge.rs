// IPC bridge: pushes serial/clipboard events from Rust to webview.
// Framebuffer is pull-based: frontend calls vm_get_frame on each rAF.

use std::time::Duration;
use tokio::time::interval;
use tauri::{AppHandle, Emitter, Manager};

use crate::machine_manager::MachineState;

#[derive(Clone, serde::Serialize)]
pub struct SerialEvent {
    pub vm_id: String,
    pub data: String,
}

#[derive(Clone, serde::Serialize)]
pub struct ClipboardEvent {
    pub vm_id: String,
    pub text: String,
}

#[derive(Clone, serde::Serialize)]
pub struct VmStatusEvent {
    pub vm_id: String,
    pub state: String,
}

pub fn start_poll_loop(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_millis(100));
        loop {
            ticker.tick().await;

            let state = match app.try_state::<MachineState>() {
                Some(s) => s,
                None    => continue,
            };

            for status in state.manager().list() {
                let id = &status.id;

                if let Some(data) = state.manager().drain_stdout(id) {
                    let _ = app.emit("vm:stdout", SerialEvent { vm_id: id.clone(), data });
                }

                if let Some(text) = state.manager().drain_clipboard(id) {
                    let _ = app.emit("vm:clipboard", ClipboardEvent { vm_id: id.clone(), text });
                }
            }
        }
    });
}
