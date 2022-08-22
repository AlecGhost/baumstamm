#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::generate_handler;
mod grid;
mod routes;

fn main() {
    tauri::Builder::default()
        .invoke_handler(generate_handler![routes::generate_grid])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
