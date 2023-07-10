#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use baumstamm_lib::FamilyTree;
use specta::{
    collect_types,
    ts::{BigIntExportBehavior, ExportConfiguration},
};
use std::sync::Mutex;
use tauri_specta::ts;

mod commands;

#[derive(Debug, Default)]
struct State(Mutex<AppState>);

#[derive(Debug, Default)]
struct AppState {
    tree: FamilyTree,
}

fn main() {
    #[cfg(debug_assertions)]
    export();

    tauri::Builder::default()
        .manage(State::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_persons,
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

#[test]
fn export_bindings() {
    #[cfg(debug_assertions)]
    export();
}

#[cfg(debug_assertions)]
fn export() {
    ts::export_with_cfg(
        collect_types![
            commands::get_persons,
            commands::add_parent,
            commands::add_child,
            commands::add_new_relationship,
            commands::add_relationship_with_partner,
            commands::insert_info,
            commands::remove_info,
            commands::display_graph,
        ]
        .unwrap(),
        ExportConfiguration::default().bigint(BigIntExportBehavior::String),
        "../src/bindings.ts",
    )
    .unwrap();
}
