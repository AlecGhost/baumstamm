#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use baumstamm_lib::FamilyTree;
use color_eyre::eyre::Result;
#[cfg(debug_assertions)]
use specta::{
    collect_types,
    ts::{BigIntExportBehavior, ExportConfiguration},
};
use std::{path::PathBuf, sync::Mutex};
use tauri::{api::dialog::FileDialogBuilder, CustomMenuItem, Manager, Menu, MenuItem, Submenu};
#[cfg(debug_assertions)]
use tauri_specta::ts;

mod commands;
mod error;

#[derive(Debug, Default)]
struct State(Mutex<AppState>);

#[derive(Debug, Default)]
struct AppState {
    tree: FamilyTree,
    path: Option<PathBuf>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    #[cfg(debug_assertions)]
    export();

    tauri::Builder::default()
        .manage(State::default())
        .menu(build_menu())
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
            commands::remove_person,
            commands::merge_person,
            commands::insert_info,
            commands::remove_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}

fn build_menu() -> Menu {
    let about_menu = Submenu::new(
        "App",
        Menu::new()
            .add_native_item(MenuItem::Hide)
            .add_native_item(MenuItem::HideOthers)
            .add_native_item(MenuItem::ShowAll)
            .add_native_item(MenuItem::Separator)
            .add_native_item(MenuItem::Quit),
    );

    let open = CustomMenuItem::new("open", "Open").accelerator("cmdOrControl+O");
    let save_as = CustomMenuItem::new("save_as", "Save As").accelerator("cmdOrControl+Shift+S");
    let file = Submenu::new("File", Menu::new().add_item(open).add_item(save_as));

    let edit_menu = Submenu::new(
        "Edit",
        Menu::new()
            .add_native_item(MenuItem::Undo)
            .add_native_item(MenuItem::Redo)
            .add_native_item(MenuItem::Separator)
            .add_native_item(MenuItem::Cut)
            .add_native_item(MenuItem::Copy)
            .add_native_item(MenuItem::Paste)
            .add_native_item(MenuItem::SelectAll),
    );

    let view_menu = Submenu::new(
        "View",
        Menu::new().add_native_item(MenuItem::EnterFullScreen),
    );

    let window_menu = Submenu::new(
        "Window",
        Menu::new()
            .add_native_item(MenuItem::Minimize)
            .add_native_item(MenuItem::Zoom),
    );

    Menu::new()
        .add_submenu(about_menu)
        .add_submenu(file)
        .add_submenu(edit_menu)
        .add_submenu(view_menu)
        .add_submenu(window_menu)
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
            commands::remove_person,
            commands::merge_person,
            commands::insert_info,
            commands::remove_info,
        ]
        .expect("type collection failed"),
        ExportConfiguration::default().bigint(BigIntExportBehavior::String),
        "../src/bindings-tauri.ts",
    )
    .expect("export failed");
}
