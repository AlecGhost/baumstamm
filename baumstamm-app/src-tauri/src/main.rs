// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::{api::dialog::FileDialogBuilder, CustomMenuItem, Manager, Menu, MenuItem, Submenu};

const EVENT_MENU_OPEN: &str = "menu-open";
const EVENT_MENU_SAVE_AS: &str = "menu-save-as";
const EVENT_OPEN: &str = "open";
const EVENT_OPEN_ERROR: &str = "open-error";
const EVENT_SAVE_AS: &str = "save-as";

fn main() {
    tauri::Builder::default()
        .menu(build_menu())
        .on_menu_event(|event| {
            let window = event.window();
            let app = window.app_handle();
            match event.menu_item_id() {
                EVENT_MENU_OPEN => {
                    FileDialogBuilder::new()
                        .add_filter("Application", &["json"])
                        .pick_file(move |path| {
                            if let Some(path) = path {
                                match std::fs::read_to_string(path) {
                                    Ok(content) => app
                                        .emit_all(EVENT_OPEN, content)
                                        .expect("open event failed"),
                                    Err(err) => app
                                        .emit_all(EVENT_OPEN_ERROR, err.to_string())
                                        .expect("open-error event failed"),
                                };
                            }
                        });
                }
                EVENT_MENU_SAVE_AS => {
                    FileDialogBuilder::new()
                        .add_filter("Application", &["json"])
                        .save_file(move |path| {
                            if let Some(path) = path {
                                app.emit_all(EVENT_SAVE_AS, path)
                                    .expect("save-as event failed");
                            }
                        });
                }
                _ => {}
            };
        })
        .invoke_handler(tauri::generate_handler![save_as])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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

    let open = CustomMenuItem::new(EVENT_MENU_OPEN, "Open").accelerator("cmdOrControl+O");
    let save_as =
        CustomMenuItem::new(EVENT_MENU_SAVE_AS, "Save As").accelerator("cmdOrControl+Shift+S");
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

#[tauri::command]
async fn save_as(path: String, content: String) -> Result<(), String> {
    std::fs::write(path, content).map_err(|err| err.to_string())
}
