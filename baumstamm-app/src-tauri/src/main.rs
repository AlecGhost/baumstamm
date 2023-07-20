#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use baumstamm_lib::FamilyTree;
use specta::{
    collect_types,
    ts::{BigIntExportBehavior, ExportConfiguration},
};
use std::{path::PathBuf, sync::Mutex};
use tauri::{api::dialog::FileDialogBuilder, CustomMenuItem, Manager, Menu, Submenu};
use tauri_specta::ts;

mod commands;
mod error;
mod grid;

#[derive(Debug, Default)]
struct State(Mutex<AppState>);

#[derive(Debug, Default)]
struct AppState {
    tree: FamilyTree,
    path: Option<PathBuf>,
}

fn main() {
    #[cfg(debug_assertions)]
    export();

    let open = CustomMenuItem::new("open".to_string(), "Open");
    let save_as = CustomMenuItem::new("save_as".to_string(), "Save As");
    let submenu = Submenu::new("File", Menu::new().add_item(open).add_item(save_as));
    let file = CustomMenuItem::new("file".to_string(), "File");
    let menu = Menu::new().add_item(file).add_submenu(submenu);

    tauri::Builder::default()
        .manage(State::default())
        .menu(menu)
        .on_menu_event(|event| {
            let window = event.window();
            let app = window.app_handle();
            match event.menu_item_id() {
                "open" => {
                    FileDialogBuilder::new()
                        .add_filter("Application", &["json"])
                        .pick_file(move |path| {
                            if let Some(path) = path {
                                let state = app.state();
                                match commands::open_file(path, state) {
                                    Ok(_) => app.emit_all("open", ()).expect("open event failed"),
                                    Err(err) => app
                                        .emit_all("open-error", err.to_string())
                                        .expect("open-error event failed"),
                                };
                            }
                        });
                }
                "save_as" => {
                    FileDialogBuilder::new()
                        .add_filter("Application", &["json"])
                        .save_file(move |path| {
                            if let Some(path) = path {
                                let state = app.state();
                                if let Err(err) = commands::save_file(path, state) {
                                    app.emit_all("save-as-error", err.to_string())
                                        .expect("save-as-error event failed");
                                }
                            }
                        });
                }
                _ => {}
            };
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_persons,
            commands::get_relationships,
            commands::get_grid,
            commands::add_parent,
            commands::add_child,
            commands::add_new_relationship,
            commands::add_relationship_with_partner,
            commands::insert_info,
            commands::remove_info,
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
            commands::get_relationships,
            commands::get_grid,
            commands::add_parent,
            commands::add_child,
            commands::add_new_relationship,
            commands::add_relationship_with_partner,
            commands::insert_info,
            commands::remove_info,
        ]
        .expect("type collection failed"),
        ExportConfiguration::default().bigint(BigIntExportBehavior::String),
        "../src/bindings.ts",
    )
    .expect("export failed");
}
