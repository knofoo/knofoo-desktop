// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    // source: https://v2.tauri.app/develop/debug/linux-graphics/
    // error: Failed to create GBM buffer of size 800x600: Invalid argument
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    app_lib::run();
}
