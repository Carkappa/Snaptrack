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
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const CAPTURE_SHORTCUT_MAC: &str = "Cmd+Shift+J";
const CAPTURE_SHORTCUT_OTHER: &str = "Ctrl+Shift+J";

pub fn default_hotkey() -> &'static str {
    if cfg!(target_os = "macos") {
        CAPTURE_SHORTCUT_MAC
    } else {
        CAPTURE_SHORTCUT_OTHER
    }
}

/// Points the capture shortcut at `accelerator`, releasing whatever was
/// registered before. Parsing and registration are both fallible - the
/// string comes from a text field, and another app may already own the
/// combination - so the caller decides what to do on failure.
pub fn register_capture_shortcut<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    accelerator: &str,
) -> Result<(), String> {
    use std::str::FromStr;

    let shortcut = Shortcut::from_str(accelerator)
        .map_err(|_| format!("'{accelerator}' isn't a shortcut this can register."))?;

    // Dropping every previous registration keeps repeated edits from
    // leaving stale shortcuts behind.
    let _ = app.global_shortcut().unregister_all();

    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| format!("{e}"))
}

fn show_and_focus_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("capture-shortcut-triggered", ());
    }
}

pub fn run() {
    tauri::Builder::default()
        // Must be registered first, before any plugin that could take a
        // lock or bind something the second copy would also want.
        //
        // Only one copy may run: a tray app with two instances gives two
        // tray icons, a global shortcut owned by whichever started first,
        // and - the part that can actually lose data - two processes each
        // reading the whole workbook and writing it back over the other.
        // A second launch surfaces the window of the copy already running
        // and exits, which is also what clicking the shortcut should do.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_and_focus_main_window(app);
        }))
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
            let handle = app.handle().clone();

            // The user's shortcut if they set one, else the platform default.
            // A stored shortcut that no longer registers (another app took it
            // since) falls back rather than leaving no way to open the window.
            let wanted = commands::get_hotkey(handle.clone());
            if let Err(e) = register_capture_shortcut(&handle, &wanted) {
                eprintln!("Could not register {wanted}: {e}");
                let fallback = default_hotkey();
                if let Err(e) = register_capture_shortcut(&handle, fallback) {
                    eprintln!("Could not register {fallback} either: {e}");
                }
            }

            // A tray-only app that starts invisible looks like it failed to
            // launch. Show the window the first time, so the shortcut can be
            // explained instead of guessed.
            if !commands::get_seen_welcome(handle.clone()) {
                show_and_focus_main_window(&handle);
            }

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
            commands::get_status_defs,
            commands::set_status_defs,
            commands::has_api_key,
            commands::get_extraction_providers,
            commands::get_model,
            commands::set_model,
            commands::screenshot_for_application,
            commands::open_screenshot,
            commands::get_hotkey,
            commands::set_hotkey,
            commands::get_seen_welcome,
            commands::set_seen_welcome,
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
            commands::insert_application_at_index,
            commands::import_applications,
            commands::pick_import_file,
            commands::export_csv,
            commands::save_screenshot,
            commands::get_extraction_method,
            commands::set_extraction_method,
            commands::local_ocr_available,
            commands::extract_with_local_ocr,
            commands::learn_ocr_hints,
            commands::extract_with_ollama,
            commands::ollama_status,
            commands::pull_ollama_model,
            commands::get_ollama_host,
            commands::set_ollama_host,
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
