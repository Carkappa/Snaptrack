use crate::models::{ExtractionResult, JobApplication, SaveResult};
use crate::{excel, extraction, keychain};
use base64::Engine;
use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "settings.json";
const EXCEL_PATH_KEY: &str = "excel_path";
const EXTRACTION_METHOD_KEY: &str = "extraction_method";
const DEFAULT_EXTRACTION_METHOD: &str = "tesseract";

fn default_excel_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let docs = app
        .path()
        .document_dir()
        .map_err(|e| format!("Could not locate the Documents folder: {e}"))?;
    Ok(docs.join("JobApplications.xlsx"))
}

fn resolve_excel_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    match store.get(EXCEL_PATH_KEY) {
        Some(value) => {
            let path_str = value
                .as_str()
                .ok_or_else(|| "Stored Excel path was not a string.".to_string())?;
            Ok(PathBuf::from(path_str))
        }
        None => default_excel_path(app),
    }
}

#[tauri::command]
pub fn get_statuses() -> Vec<&'static str> {
    crate::models::STATUSES.to_vec()
}

#[tauri::command]
pub fn has_api_key() -> bool {
    keychain::has_api_key()
}

#[tauri::command]
pub fn save_api_key(key: String) -> Result<(), String> {
    keychain::set_api_key(&key)
}

#[tauri::command]
pub fn delete_api_key() -> Result<(), String> {
    keychain::delete_api_key()
}

#[tauri::command]
pub async fn extract_from_image(
    image_base64: String,
    media_type: String,
) -> Result<ExtractionResult, String> {
    let api_key = keychain::get_api_key()?;
    extraction::extract_fields_from_image(&api_key, &image_base64, &media_type).await
}

#[tauri::command]
pub fn get_extraction_method<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    Ok(store
        .get(EXTRACTION_METHOD_KEY)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| DEFAULT_EXTRACTION_METHOD.to_string()))
}

#[tauri::command]
pub fn set_extraction_method<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    method: String,
) -> Result<(), String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    store.set(EXTRACTION_METHOD_KEY, serde_json::json!(method));
    store
        .save()
        .map_err(|e| format!("Could not persist settings: {e}"))
}

#[tauri::command]
pub fn local_ocr_available() -> bool {
    crate::local_ocr::tesseract_available()
}

/// Free, offline alternative to `extract_from_image`, backed by a
/// locally installed Tesseract binary. Always returns `Parsed` (never
/// invents values it isn't reasonably confident about) with the full
/// raw OCR text attached to `notes` for the user to double-check, since
/// this is meaningfully less reliable than Claude's actual
/// understanding of the image.
#[tauri::command]
pub fn extract_with_local_ocr(image_base64: String) -> Result<ExtractionResult, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64)
        .map_err(|e| format!("Invalid image data: {e}"))?;

    let lines = crate::local_ocr::run_ocr(&bytes)?;
    if lines.is_empty() {
        return Ok(ExtractionResult::ParseFailed {
            raw_text: String::new(),
            error: "No text was detected in the image.".to_string(),
        });
    }

    Ok(ExtractionResult::Parsed {
        fields: crate::local_ocr::guess_fields(&lines),
    })
}

#[tauri::command]
pub fn get_excel_path<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    let path = resolve_excel_path(&app)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_excel_path<R: tauri::Runtime>(app: tauri::AppHandle<R>, path: String) -> Result<(), String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    store.set(EXCEL_PATH_KEY, serde_json::json!(path));
    store
        .save()
        .map_err(|e| format!("Could not persist settings: {e}"))
}

#[tauri::command]
pub async fn pick_excel_path<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let default_path = default_excel_path(&app).ok();
    let (tx, rx) = std::sync::mpsc::channel();
    let mut builder = app
        .dialog()
        .file()
        .add_filter("Excel Workbook", &["xlsx"])
        .set_file_name("JobApplications.xlsx");
    if let Some(default_path) = default_path {
        if let Some(parent) = default_path.parent() {
            builder = builder.set_directory(parent);
        }
    }
    builder.save_file(move |file_path| {
        let _ = tx.send(file_path);
    });
    let result = rx
        .recv()
        .map_err(|e| format!("File dialog failed: {e}"))?;
    Ok(result.map(|p| p.to_string()))
}

#[tauri::command]
pub async fn pick_image_file<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
        .pick_file(move |file_path| {
            let _ = tx.send(file_path);
        });
    let result = rx
        .recv()
        .map_err(|e| format!("File dialog failed: {e}"))?;
    Ok(result.map(|p| p.to_string()))
}

#[derive(serde::Serialize)]
pub struct ImagePayload {
    pub base64: String,
    pub media_type: String,
}

#[tauri::command]
pub fn read_image_file(path: String) -> Result<ImagePayload, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("Could not read '{path}': {e}"))?;
    let media_type = match image::guess_format(&bytes) {
        Ok(image::ImageFormat::Png) => "image/png",
        Ok(image::ImageFormat::Jpeg) => "image/jpeg",
        Ok(image::ImageFormat::WebP) => "image/webp",
        Ok(image::ImageFormat::Gif) => "image/gif",
        _ => return Err("Unsupported image format.".to_string()),
    };
    let base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(ImagePayload {
        base64,
        media_type: media_type.to_string(),
    })
}

/// Reads the current image directly from the OS clipboard via the
/// clipboard-manager plugin's Rust API (not a JS paste/clipboardData
/// event), encodes it to PNG, and returns it as base64.
#[tauri::command]
pub fn read_clipboard_image<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<Option<String>, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;

    let image = match app.clipboard().read_image() {
        Ok(img) => img,
        Err(_) => return Ok(None),
    };

    let width = image.width();
    let height = image.height();
    let rgba = image.rgba();

    let buffer = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or_else(|| "Clipboard image had an unexpected pixel format.".to_string())?;

    let mut png_bytes: Vec<u8> = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut png_bytes);
        image::DynamicImage::ImageRgba8(buffer)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| format!("Could not encode clipboard image: {e}"))?;
    }

    Ok(Some(base64::engine::general_purpose::STANDARD.encode(&png_bytes)))
}

#[tauri::command]
pub fn list_applications<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<Vec<JobApplication>, String> {
    let path = resolve_excel_path(&app)?;
    excel::read_applications(&path)
}

#[tauri::command]
pub fn save_application<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    application: JobApplication,
    force: bool,
) -> Result<SaveResult, String> {
    let path = resolve_excel_path(&app)?;
    let mut rows = excel::read_applications(&path)?;

    if !force {
        let key = application.dedupe_key();
        if let Some(existing) = rows.iter().find(|r| r.dedupe_key() == key) {
            return Ok(SaveResult::Duplicate {
                existing_status: existing.status.clone(),
            });
        }
    }

    rows.push(application);
    excel::write_applications(&path, &rows)?;
    Ok(SaveResult::Saved)
}

#[tauri::command]
pub fn update_existing_status<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    company: String,
    position: String,
    status: String,
) -> Result<(), String> {
    let path = resolve_excel_path(&app)?;
    let mut rows = excel::read_applications(&path)?;

    let key = (company.trim().to_lowercase(), position.trim().to_lowercase());
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut found = false;
    for row in rows.iter_mut() {
        if row.dedupe_key() == key {
            row.status = status.clone();
            row.last_updated = today.clone();
            found = true;
            break;
        }
    }

    if !found {
        return Err("Could not find a matching existing row to update.".to_string());
    }

    excel::write_applications(&path, &rows)
}

#[tauri::command]
pub fn update_status_at_index<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    index: usize,
    status: String,
) -> Result<(), String> {
    let path = resolve_excel_path(&app)?;
    let mut rows = excel::read_applications(&path)?;

    let row = rows
        .get_mut(index)
        .ok_or_else(|| "That row no longer exists - reload the list and try again.".to_string())?;
    row.status = status;
    row.last_updated = chrono::Local::now().format("%Y-%m-%d").to_string();

    excel::write_applications(&path, &rows)
}

/// Overwrites a row in place (edit flow from the Applications list),
/// as opposed to `save_application` which appends a new row.
#[tauri::command]
pub fn update_application_at_index<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    index: usize,
    application: JobApplication,
) -> Result<(), String> {
    let path = resolve_excel_path(&app)?;
    let mut rows = excel::read_applications(&path)?;

    let row = rows
        .get_mut(index)
        .ok_or_else(|| "That row no longer exists - reload the list and try again.".to_string())?;
    *row = application;

    excel::write_applications(&path, &rows)
}

#[tauri::command]
pub fn export_csv<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    let path = resolve_excel_path(&app)?;
    let rows = excel::read_applications(&path)?;
    let csv_path = excel::export_csv(&path, &rows)?;
    Ok(csv_path.to_string_lossy().to_string())
}

fn sanitize_for_filename(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == ' ' { c } else { '_' })
        .collect();
    let trimmed = cleaned.trim();
    let truncated: String = trimmed.chars().take(60).collect();
    if truncated.is_empty() {
        "untitled".to_string()
    } else {
        truncated.replace(' ', "_")
    }
}

/// Saves a copy of the screenshot a capture was made from into a
/// `<workbook dir>/JobApplications_screenshots/` folder, named after the
/// company/position/date, purely for the user's own reference - it is
/// not linked from any Excel column.
fn extension_for_media_type(media_type: &str) -> &'static str {
    match media_type {
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "png",
    }
}

/// Saves a copy of the screenshot a capture was made from into a
/// `<workbook dir>/JobApplications_screenshots/` folder, named after the
/// company/position/date, purely for the user's own reference - it is
/// not linked from any Excel column.
#[tauri::command]
pub fn save_screenshot<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    company: String,
    position: String,
    date_applied: String,
    image_base64: String,
    media_type: String,
) -> Result<String, String> {
    let excel_path = resolve_excel_path(&app)?;
    let parent = excel_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| "Workbook has no parent directory.".to_string())?;

    let screenshots_dir = parent.join("JobApplications_screenshots");
    std::fs::create_dir_all(&screenshots_dir)
        .map_err(|e| format!("Could not create '{}': {e}", screenshots_dir.display()))?;

    let file_name = format!(
        "{}_{}_{}.{}",
        sanitize_for_filename(&date_applied),
        sanitize_for_filename(&company),
        sanitize_for_filename(&position),
        extension_for_media_type(&media_type)
    );
    let file_path = screenshots_dir.join(file_name);

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64)
        .map_err(|e| format!("Invalid image data: {e}"))?;
    std::fs::write(&file_path, bytes)
        .map_err(|e| format!("Could not save screenshot to '{}': {e}", file_path.display()))?;

    Ok(file_path.to_string_lossy().to_string())
}
