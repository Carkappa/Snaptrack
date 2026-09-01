pub mod commands;
mod excel;
mod extraction;
mod keychain;
mod local_ocr;
pub mod models;
pub mod updates;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const CAPTURE_SHORTCUT_MAC: &str = "Cmd+Shift+J";
const CAPTURE_SHORTCUT_OTHER: &str = "Ctrl+Shift+J";

fn show_and_focus_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("capture-shortcut-triggered", ());
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        show_and_focus_main_window(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let shortcut_str = if cfg!(target_os = "macos") {
                CAPTURE_SHORTCUT_MAC
            } else {
                CAPTURE_SHORTCUT_OTHER
            };
            let shortcut: Shortcut = if cfg!(target_os = "macos") {
                Shortcut::new(
                    Some(Modifiers::SUPER | Modifiers::SHIFT),
                    Code::KeyJ,
                )
            } else {
                Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyJ)
            };
            app.global_shortcut()
                .register(shortcut)
                .unwrap_or_else(|e| eprintln!("Could not register {shortcut_str}: {e}"));

            let show_item = MenuItem::with_id(app, "show", "Open Job Tracker", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => show_and_focus_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_and_focus_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window hides it instead of quitting the app -
            // the app keeps running in the tray until "Quit" is chosen.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_statuses,
            commands::has_api_key,
            commands::save_api_key,
            commands::delete_api_key,
            commands::extract_from_image,
            commands::get_excel_path,
            commands::set_excel_path,
            commands::pick_excel_path,
            commands::pick_image_file,
            commands::read_image_file,
            commands::read_clipboard_image,
            commands::list_applications,
            commands::save_application,
            commands::update_existing_status,
            commands::update_status_at_index,
            commands::update_application_at_index,
            commands::delete_application_at_index,
            commands::export_csv,
            commands::save_screenshot,
            commands::get_extraction_method,
            commands::set_extraction_method,
            commands::local_ocr_available,
            commands::extract_with_local_ocr,
            commands::get_app_version,
            commands::get_update_check_enabled,
            commands::set_update_check_enabled,
            commands::get_auto_install_updates,
            commands::set_auto_install_updates,
            commands::check_for_update,
            commands::install_update,
            commands::open_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the job tracker application");
}
