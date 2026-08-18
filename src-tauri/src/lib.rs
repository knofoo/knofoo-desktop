#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

mod hypervisor;
mod virtio;
mod net;
mod machine_manager;
mod ipc_bridge;

use std::sync::Arc;
use machine_manager::MachineState;
use crate::hypervisor::{FrameSnapshot, MachineConfig, VmStatus};
use crate::virtio::input::InputEvent;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// ── Machine / VM commands ─────────────────────────────────────────────────────

#[tauri::command]
async fn vm_start(
    state: tauri::State<'_, MachineState>,
    config: MachineConfig,
) -> Result<(), String> {
    let manager = Arc::clone(&state.inner().0);
    tauri::async_runtime::spawn_blocking(move || manager.start(config))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn vm_stop(
    state: tauri::State<'_, MachineState>,
    id: String,
) -> Result<(), String> {
    state.inner().manager().stop(&id)
}

#[tauri::command]
fn vm_pause(
    state: tauri::State<'_, MachineState>,
    id: String,
) -> Result<(), String> {
    state.inner().manager().pause(&id)
}

#[tauri::command]
fn vm_resume(
    state: tauri::State<'_, MachineState>,
    id: String,
) -> Result<(), String> {
    state.inner().manager().resume(&id)
}

#[tauri::command]
fn vm_input(
    state: tauri::State<'_, MachineState>,
    id: String,
    event: InputEvent,
) -> Result<(), String> {
    state.inner().manager().send_input(&id, event)
}

#[tauri::command]
fn vm_send_serial(
    state: tauri::State<'_, MachineState>,
    id: String,
    data: String,
) -> Result<(), String> {
    state.inner().manager().send_serial(&id, data.as_bytes())
}

#[tauri::command]
fn vm_clipboard_to_vm(
    state: tauri::State<'_, MachineState>,
    id: String,
    text: String,
) -> Result<(), String> {
    state.inner().manager().send_clipboard_to_vm(&id, &text)
}

#[tauri::command]
fn vm_get_frame(
    state: tauri::State<'_, MachineState>,
    id: String,
) -> Result<Option<FrameSnapshot>, String> {
    Ok(state.inner().manager().get_frame(&id))
}

#[tauri::command]
fn vm_get_frame_bin(
    state: tauri::State<'_, MachineState>,
    id: String,
) -> tauri::ipc::Response {
    // Returns raw bytes: [w:u32 LE | h:u32 LE | rgba ...].
    // Empty body = no new frame. Skips serde + base64 entirely.
    tauri::ipc::Response::new(state.inner().manager().get_frame_bin(&id))
}

#[tauri::command]
fn vm_status(
    state: tauri::State<'_, MachineState>,
    id: String,
) -> Result<Option<VmStatus>, String> {
    Ok(state.inner().manager().status(&id))
}

#[tauri::command]
fn vm_list(
    state: tauri::State<'_, MachineState>,
) -> Result<Vec<VmStatus>, String> {
    Ok(state.inner().manager().list())
}

#[tauri::command]
fn vm_diag(
    state: tauri::State<'_, MachineState>,
    id: String,
) -> Result<String, String> {
    Ok(state.inner().manager().diag(&id))
}

#[tauri::command]
fn vm_ensure_disk(vault_path: String, vm_id: String, size_mb: u64) -> Result<String, String> {
    let disk_dir = std::path::Path::new(&vault_path)
        .join(".knofoo")
        .join("machines")
        .join(&vm_id);
    std::fs::create_dir_all(&disk_dir).map_err(|e| e.to_string())?;
    let disk_path = disk_dir.join("disk.img");
    if !disk_path.exists() {
        crate::virtio::blk::BlkDevice::open_or_create_qcow2(
            disk_path.to_str().unwrap(),
            size_mb,
        )?;
    }
    Ok(disk_path.to_string_lossy().into_owned())
}

const DEFAULT_CONFIG: &str = r#"{
  "knofoo": "0.1.0",
  "paths": {
    "graphs": ".knofoo/graphs",
    "modules": ".knofoo/modules",
    "notes": ".knofoo/notes",
    "assets": ".knofoo/assets"
  }
}
"#;

#[tauri::command]
fn init_vault(path: &str) -> Result<(), String> {
    let base = std::path::Path::new(path).join(".knofoo");
    for sub in &["graphs", "modules", "notes", "assets", "keys", "validators", "certs"] {
        std::fs::create_dir_all(base.join(sub)).map_err(|e| e.to_string())?;
    }
    let config_path = base.join("config.json");
    if !config_path.exists() {
        std::fs::write(&config_path, DEFAULT_CONFIG).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn read_config(vault_path: &str) -> Result<String, String> {
    let config_path = std::path::Path::new(vault_path).join(".knofoo").join("config.json");
    if !config_path.exists() {
        return Ok(DEFAULT_CONFIG.to_string());
    }
    std::fs::read_to_string(&config_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_config(vault_path: &str, config: &str) -> Result<(), String> {
    let config_path = std::path::Path::new(vault_path).join(".knofoo").join("config.json");
    std::fs::write(&config_path, config).map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    children: Option<Vec<FileEntry>>,
}

fn list_dir(dir_path: &str, recursive: bool) -> Result<Vec<FileEntry>, String> {
    let dir = std::path::Path::new(dir_path);
    if !dir.exists() { return Ok(vec![]); }
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    let mut result: Vec<FileEntry> = entries
        .flatten()
        .filter_map(|e| {
            let name   = e.file_name().to_string_lossy().into_owned();
            let path   = e.path().to_string_lossy().into_owned();
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let children = if recursive && is_dir {
                list_dir(&path, true).ok()
            } else {
                None
            };
            Some(FileEntry { name, path, is_dir, children })
        })
        .collect();
    result.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });
    Ok(result)
}

// Find next available "Untitled.json", "Untitled 1.json", etc.
fn next_untitled(dir: &std::path::Path) -> String {
    let base = dir.join("Untitled.json");
    if !base.exists() { return "Untitled".to_string(); }
    let mut n = 1u32;
    loop {
        let candidate = dir.join(format!("Untitled {}.json", n));
        if !candidate.exists() { return format!("Untitled {}", n); }
        n += 1;
    }
}

fn create_item(dir_path: &str, name: Option<&str>, kind: &str) -> Result<String, String> {
    let dir = std::path::Path::new(dir_path);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let resolved = match name {
        Some(n) => {
            let safe = n.trim().replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
            if safe.is_empty() { return Err("Name cannot be empty".into()); }
            safe
        }
        None => next_untitled(dir),
    };

    let file_path = dir.join(format!("{}.json", resolved));
    if file_path.exists() {
        return Err(format!("{}.json already exists", resolved));
    }
    let empty = format!(r#"{{"knofoo":"0.1.0","kind":"{}","nodes":[],"edges":[]}}"#, kind);
    std::fs::write(&file_path, empty).map_err(|e| e.to_string())?;
    Ok(file_path.to_string_lossy().into_owned())
}

fn rename_item(dir_path: &str, old_name: &str, new_name: &str) -> Result<(), String> {
    let new_name = new_name.trim();
    if new_name.is_empty() { return Err("Name cannot be empty".into()); }
    let dir = std::path::Path::new(dir_path);
    let src = dir.join(old_name);
    let dst = dir.join(new_name);
    if dst.exists() { return Err(format!("{} already exists", new_name)); }
    std::fs::rename(src, dst).map_err(|e| e.to_string())
}

fn delete_item(dir_path: &str, name: &str) -> Result<(), String> {
    let path = std::path::Path::new(dir_path).join(name);
    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(path).map_err(|e| e.to_string())
    }
}

// ── Generic rename / delete ───────────────────────────────────────────────────

#[tauri::command]
fn rename_entry(dir_path: &str, old_name: &str, new_name: &str) -> Result<(), String> {
    rename_item(dir_path, old_name, new_name)
}

#[tauri::command]
fn delete_entry(dir_path: &str, name: &str) -> Result<(), String> {
    delete_item(dir_path, name)
}

// ── Vault file/folder commands ────────────────────────────────────────────────

fn next_untitled_plain(dir: &std::path::Path, ext: &str) -> String {
    let base_name = if ext.is_empty() { "Untitled".to_string() } else { format!("Untitled.{}", ext) };
    if !dir.join(&base_name).exists() {
        return if ext.is_empty() { "Untitled".to_string() } else { format!("Untitled.{}", ext) };
    }
    let mut n = 1u32;
    loop {
        let candidate = if ext.is_empty() {
            format!("Untitled {}", n)
        } else {
            format!("Untitled {}.{}", n, ext)
        };
        if !dir.join(&candidate).exists() { return candidate; }
        n += 1;
    }
}

#[tauri::command]
fn create_vault_file(vault_path: &str) -> Result<String, String> {
    let dir = std::path::Path::new(vault_path);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let name = next_untitled_plain(dir, "md");
    let file_path = dir.join(&name);
    std::fs::write(&file_path, "").map_err(|e| e.to_string())?;
    Ok(file_path.to_string_lossy().into_owned())
}

#[tauri::command]
fn create_vault_folder(vault_path: &str) -> Result<String, String> {
    let dir = std::path::Path::new(vault_path);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let name = next_untitled_plain(dir, "");
    let folder_path = dir.join(&name);
    std::fs::create_dir(&folder_path).map_err(|e| e.to_string())?;
    Ok(folder_path.to_string_lossy().into_owned())
}

#[tauri::command]
fn read_file(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_file(path: &str, content: &str) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn rename_path(src: &str, dst: &str) -> Result<(), String> {
    std::fs::rename(src, dst).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_path(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    if p.is_dir() {
        std::fs::remove_dir_all(p).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(p).map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn mkdir_path(path: &str) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn exists_path(path: &str) -> Result<bool, String> {
    Ok(std::path::Path::new(path).exists())
}

// ── Graph commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn list_graphs(vault_path: &str) -> Result<Vec<FileEntry>, String> {
    list_dir(vault_path, false)
}

#[tauri::command]
fn create_graph(vault_path: &str, name: Option<&str>) -> Result<String, String> {
    create_item(vault_path, name, "graph")
}

#[tauri::command]
fn rename_graph(vault_path: &str, old_name: &str, new_name: &str) -> Result<(), String> {
    rename_item(vault_path, old_name, new_name)
}

#[tauri::command]
fn delete_graph(vault_path: &str, name: &str) -> Result<(), String> {
    delete_item(vault_path, name)
}

#[tauri::command]
fn list_dir_recursive(vault_path: &str) -> Result<Vec<FileEntry>, String> {
    list_dir(vault_path, true)
}

// ── Module commands ───────────────────────────────────────────────────────────

#[tauri::command]
fn list_modules(vault_path: &str) -> Result<Vec<FileEntry>, String> {
    list_dir(vault_path, false)
}

#[tauri::command]
fn create_module(vault_path: &str, name: Option<&str>) -> Result<String, String> {
    create_item(vault_path, name, "module")
}

#[tauri::command]
fn rename_module(vault_path: &str, old_name: &str, new_name: &str) -> Result<(), String> {
    rename_item(vault_path, old_name, new_name)
}

#[tauri::command]
fn delete_module(vault_path: &str, name: &str) -> Result<(), String> {
    delete_item(vault_path, name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(MachineState::new())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            init_vault,
            read_config,
            write_config,
            read_file,
            write_file,
            rename_path,
            remove_path,
            mkdir_path,
            exists_path,
            create_vault_file,
            create_vault_folder,
            list_dir_recursive,
            rename_entry,
            delete_entry,
            list_graphs,
            create_graph,
            rename_graph,
            delete_graph,
            list_modules,
            create_module,
            rename_module,
            delete_module,
            // VM commands
            vm_start,
            vm_stop,
            vm_pause,
            vm_resume,
            vm_input,
            vm_send_serial,
            vm_clipboard_to_vm,
            vm_get_frame,
            vm_get_frame_bin,
            vm_status,
            vm_list,
            vm_diag,
            vm_ensure_disk,
        ])
        .setup(|app| {
            ipc_bridge::start_poll_loop(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
