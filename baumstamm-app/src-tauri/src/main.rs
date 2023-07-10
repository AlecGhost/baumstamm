#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use baumstamm_lib::FamilyTree;
use std::sync::Mutex;

mod commands;

#[derive(Debug, Default)]
struct State(Mutex<AppState>);

#[derive(Debug, Default)]
struct AppState {
    tree: FamilyTree,
}

fn main() {
    tauri::Builder::default()
        .manage(State::default())
        .invoke_handler(tauri::generate_handler![
            commands::add_parent,
            commands::add_child,
            commands::add_new_relationship,
            commands::add_relationship_with_partner,
            commands::insert_info,
            commands::remove_info,
            commands::display_graph,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
