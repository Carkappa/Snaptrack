use crate::models::{ExtractionResult, JobApplication, SaveResult, StatusDef};
use crate::updates::UpdateCheck;
use crate::{excel, extraction, keychain, updates};
use base64::Engine;
use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "settings.json";
const EXCEL_PATH_KEY: &str = "excel_path";
const EXTRACTION_METHOD_KEY: &str = "extraction_method";
/// The method a machine with nothing stored should use.
///
/// The OS engine where there is one, because it needs no install and no
/// key - which was the whole reason for adding it. Defaulting to Tesseract
/// instead meant a new user's first capture failed with "Tesseract not
/// found" while a perfectly good engine sat unused in their operating
/// system. Only machines without one (Linux) still start on Tesseract.
///
/// Anyone who has picked a method has it stored, so this changes nothing
/// for them.
fn default_extraction_method() -> String {
    if crate::system_ocr::available() {
        "system".to_string()
    } else {
        crate::models::DEFAULT_PROVIDER.to_string()
    }
}
const HOTKEY_KEY: &str = "capture_hotkey";
const SEEN_WELCOME_KEY: &str = "seen_welcome";
const STATUS_DEFS_KEY: &str = "status_defs";
const MODELS_KEY: &str = "provider_models";
const OCR_HINTS_KEY: &str = "ocr_field_hints";
const OLLAMA_HOST_KEY: &str = "ollama_host";
const FALLBACK_CHAIN_KEY: &str = "extraction_fallbacks";
const OLLAMA_UNLOAD_KEY: &str = "ollama_unload_after_use";
const DEFAULT_OLLAMA_HOST: &str = "http://localhost:11434";
const UPDATE_CHECK_ENABLED_KEY: &str = "update_check_enabled";
const AUTO_INSTALL_UPDATES_KEY: &str = "auto_install_updates";
const LAST_UPDATE_CHECK_KEY: &str = "last_update_check";

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

/// The user's status list, falling back to the built-in six. A stored list
/// that no longer parses (hand-edited settings file, an older shape) is
/// ignored rather than fatal - the app still has to open.
fn resolve_status_defs<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Vec<StatusDef> {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(STATUS_DEFS_KEY))
        .and_then(|value| serde_json::from_value::<Vec<StatusDef>>(value).ok())
        .and_then(|defs| crate::models::sanitize_status_defs(defs).ok())
        .unwrap_or_else(crate::models::default_status_defs)
}

fn status_names<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Vec<String> {
    resolve_status_defs(app)
        .into_iter()
        .map(|d| d.name)
        .collect()
}

#[tauri::command]
pub fn get_statuses<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Vec<String> {
    status_names(&app)
}

#[tauri::command]
pub fn get_status_defs<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Vec<StatusDef> {
    resolve_status_defs(&app)
}

/// Replaces the status list.
///
/// Rows already carrying a status that has just been removed are left
/// exactly as they are - the workbook is the user's record, and silently
/// rewriting their history to fit a settings change would be worse than
/// showing a status that is no longer offered for new entries.
#[tauri::command]
pub fn set_status_defs<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    defs: Vec<StatusDef>,
) -> Result<Vec<StatusDef>, String> {
    let cleaned = crate::models::sanitize_status_defs(defs)?;
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    store.set(
        STATUS_DEFS_KEY,
        serde_json::to_value(&cleaned).map_err(|e| e.to_string())?,
    );
    store
        .save()
        .map_err(|e| format!("Could not persist settings: {e}"))?;
    Ok(cleaned)
}

/// Everything the Settings tab needs to render the method dropdown and the
/// key card, including which providers have a key stored.
#[tauri::command]
pub fn get_extraction_providers() -> Vec<crate::models::ExtractionProvider> {
    crate::models::extraction_providers()
}

/// The model in force for a provider: the user's override if they set one,
/// otherwise the shipped default. An override that has been blanked falls
/// back rather than sending an empty model name.
fn resolve_model<R: tauri::Runtime>(app: &tauri::AppHandle<R>, provider: &str) -> String {
    let default = crate::models::provider_or_default(provider).default_model;
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(MODELS_KEY))
        .and_then(|v| v.get(provider).and_then(|m| m.as_str()).map(str::to_string))
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .unwrap_or(default)
}

#[tauri::command]
pub fn get_model<R: tauri::Runtime>(app: tauri::AppHandle<R>, provider: String) -> String {
    resolve_model(&app, &provider)
}

/// Sets the model for a provider. An empty value clears the override and
/// goes back to the shipped default, which is the way out if someone types
/// a model name that doesn't exist.
#[tauri::command]
pub fn set_model<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    provider: String,
    model: String,
) -> Result<String, String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    let mut all = store
        .get(MODELS_KEY)
        .and_then(|v| serde_json::from_value::<std::collections::HashMap<String, String>>(v).ok())
        .unwrap_or_default();

    let trimmed = model.trim().to_string();
    if trimmed.is_empty() {
        all.remove(&provider);
    } else {
        all.insert(provider.clone(), trimmed);
    }

    store.set(
        MODELS_KEY,
        serde_json::to_value(&all).map_err(|e| e.to_string())?,
    );
    store
        .save()
        .map_err(|e| format!("Could not persist settings: {e}"))?;
    Ok(resolve_model(&app, &provider))
}

#[tauri::command]
pub fn has_api_key(provider: String) -> bool {
    keychain::has_api_key(&provider)
}

#[tauri::command]
pub fn save_api_key(provider: String, key: String) -> Result<(), String> {
    keychain::set_api_key(&provider, &key)
}

#[tauri::command]
pub fn delete_api_key(provider: String) -> Result<(), String> {
    keychain::delete_api_key(&provider)
}

#[tauri::command]
pub async fn extract_from_image<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    image_base64: String,
    media_type: String,
) -> Result<ExtractionResult, String> {
    let method = get_extraction_method(app.clone())?;
    run_cloud(&app, &method, &image_base64, &media_type).await
}

/// Runs one named cloud provider.
///
/// Takes the provider rather than reading the stored method, because the
/// fallback chain needs to run one that is not the stored method. Reading
/// it here meant a cloud fallback silently re-ran the primary with the
/// primary's key and reported the failure under the fallback's name.
async fn run_cloud<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    method: &str,
    image_base64: &str,
    media_type: &str,
) -> Result<ExtractionResult, String> {
    let provider = crate::models::provider_or_default(method);
    if !provider.needs_key {
        return Err(format!(
            "{} does not use an API key - this command is for the cloud providers.",
            provider.label
        ));
    }
    let api_key = keychain::get_api_key(&provider.id)?;
    let model = resolve_model(app, &provider.id);
    extraction::extract_fields_from_image(
        &provider.id,
        &model,
        &api_key,
        image_base64,
        media_type,
        &provider.api_base,
    )
    .await
}

#[tauri::command]
pub fn get_extraction_method<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    Ok(store
        .get(EXTRACTION_METHOD_KEY)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(default_extraction_method))
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

/// Whether this machine has an OCR engine built into the operating
/// system, so the method is only offered where it can actually run.
#[tauri::command]
pub fn system_ocr_available() -> bool {
    crate::system_ocr::available()
}

/// Reads a screenshot with the operating system's own engine.
///
/// Returns the same shape the Tesseract path does, so click-to-fill and
/// learning from corrections work here unchanged.
#[tauri::command]
pub async fn extract_with_system_ocr<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    image_base64: String,
) -> Result<LocalOcrResult, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64)
        .map_err(|e| format!("Invalid image data: {e}"))?;

    let lines = crate::system_ocr::run(&bytes).await?;
    let blocks: Vec<String> = lines.iter().map(|l| l.text.trim().to_string()).collect();
    let mut fields = crate::local_ocr::guess_fields(&lines);

    let site = crate::local_ocr::detect_site(&blocks.join("\n"));
    if let Some(site) = site {
        let hints = read_hints(&app, site);
        crate::local_ocr::apply_hints(&mut fields, &blocks, &hints);
    }

    Ok(LocalOcrResult {
        result: ExtractionResult::Parsed { fields },
        blocks,
        site: site.map(str::to_string),
    })
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
pub fn extract_with_local_ocr<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    image_base64: String,
) -> Result<LocalOcrResult, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64)
        .map_err(|e| format!("Invalid image data: {e}"))?;

    let lines = crate::local_ocr::run_ocr(&bytes)?;
    if lines.is_empty() {
        return Ok(LocalOcrResult {
            result: ExtractionResult::ParseFailed {
                raw_text: String::new(),
                error: "No text was detected in the image.".to_string(),
            },
            blocks: Vec::new(),
            site: None,
        });
    }

    let blocks: Vec<String> = lines.iter().map(|l| l.text.trim().to_string()).collect();
    let mut fields = crate::local_ocr::guess_fields(&lines);

    // What the user corrected last time on this board beats the layout
    // heuristics, which have no way of knowing this page is unusual.
    let site = crate::local_ocr::detect_site(&blocks.join("\n"));
    if let Some(site) = site {
        let hints = read_hints(&app, site);
        crate::local_ocr::apply_hints(&mut fields, &blocks, &hints);
    }

    Ok(LocalOcrResult {
        result: ExtractionResult::Parsed { fields },
        blocks,
        site: site.map(str::to_string),
    })
}

/// The models the app offers, each marked with whether it is downloaded.
#[tauri::command]
pub async fn ollama_models<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Vec<OllamaCatalogueEntry> {
    let host = resolve_ollama_host(&app);
    let installed = installed_models(&host).await.unwrap_or_default();
    crate::models::ollama_catalogue()
        .into_iter()
        .map(|info| {
            let family = |m: &str| m.split(':').next().unwrap_or(m).to_lowercase();
            let downloaded = installed
                .iter()
                .any(|m| m == &info.id || family(m) == family(&info.id));
            OllamaCatalogueEntry { info, downloaded }
        })
        .collect()
}

#[derive(serde::Serialize)]
pub struct OllamaCatalogueEntry {
    #[serde(flatten)]
    pub info: crate::models::OllamaModelInfo,
    pub downloaded: bool,
}

#[tauri::command]
pub fn get_ollama_host<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> String {
    resolve_ollama_host(&app)
}

/// Whether to hand the model back to the operating system after each
/// capture. On by default: several gigabytes held between captures that are
/// hours apart is not a trade most people would choose, and it is the
/// opposite of how the rest of the app behaves.
fn resolve_unload_after_use<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> bool {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(OLLAMA_UNLOAD_KEY))
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

#[tauri::command]
pub fn get_ollama_unload<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> bool {
    resolve_unload_after_use(&app)
}

#[tauri::command]
pub fn set_ollama_unload<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    enabled: bool,
) -> Result<(), String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    store.set(OLLAMA_UNLOAD_KEY, serde_json::json!(enabled));
    store
        .save()
        .map_err(|e| format!("Could not persist settings: {e}"))
}

fn resolve_ollama_host<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> String {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(OLLAMA_HOST_KEY))
        .and_then(|v| v.as_str().map(str::to_string))
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| DEFAULT_OLLAMA_HOST.to_string())
}

/// Blank goes back to the default, which is the way out of a typo'd host.
#[tauri::command]
pub fn set_ollama_host<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    host: String,
) -> Result<String, String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    let trimmed = host.trim().to_string();
    if trimmed.is_empty() {
        store.delete(OLLAMA_HOST_KEY);
    } else {
        store.set(OLLAMA_HOST_KEY, serde_json::json!(trimmed));
    }
    store
        .save()
        .map_err(|e| format!("Could not persist settings: {e}"))?;
    Ok(resolve_ollama_host(&app))
}

/// Whether Ollama is running, and which models it has pulled - so Settings
/// can say "running, but you have not pulled that model" rather than
/// failing at capture time.
/// What Ollama has pulled, or None when it isn't reachable at all - which
/// is a different problem from having no models, and gets a different
/// message.
async fn installed_models(host: &str) -> Option<Vec<String>> {
    let url = format!("{}/api/tags", host.trim_end_matches('/'));
    let response = reqwest::Client::new().get(&url).send().await.ok()?;
    let body = response.json::<serde_json::Value>().await.ok()?;
    Some(
        body.get("models")?
            .as_array()?
            .iter()
            .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(str::to_string))
            .collect(),
    )
}

#[tauri::command]
pub async fn ollama_status<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> OllamaStatus {
    let host = resolve_ollama_host(&app);
    let preferred = resolve_model(&app, "ollama");
    let Some(models) = installed_models(&host).await else {
        return OllamaStatus {
            running: false,
            models: Vec::new(),
            model: preferred,
            model_ready: false,
            recommended: crate::models::provider_or_default("ollama").default_model,
        };
    };

    // Whatever is already pulled beats making the user download something.
    let chosen = crate::models::best_available_model(&preferred, &models);
    // Only models that can actually be picked are offered: an embedding
    // model in the list is a trap, since choosing it fails at capture time.
    let models = models
        .into_iter()
        .filter(|m| !m.to_lowercase().contains("embed"))
        .collect();
    OllamaStatus {
        running: true,
        model_ready: chosen.is_some(),
        model: chosen.unwrap_or_else(|| preferred.clone()),
        models,
        recommended: crate::models::provider_or_default("ollama").default_model,
    }
}

#[derive(serde::Serialize)]
pub struct OllamaStatus {
    pub running: bool,
    pub models: Vec<String>,
    /// The model that would actually be used right now.
    pub model: String,
    /// False when nothing usable is pulled, so the UI offers to fetch one.
    pub model_ready: bool,
    /// What to pull when there is nothing.
    pub recommended: String,
}

/// Downloads a model, reporting progress, so nobody has to open a terminal.
///
/// Ollama streams newline-delimited JSON for this; the bytes arrive in
/// arbitrary chunks, so lines are reassembled across chunk boundaries
/// rather than assuming one chunk is one line.
#[tauri::command]
pub async fn pull_ollama_model<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    model: String,
) -> Result<(), String> {
    use tauri::Emitter;

    let host = resolve_ollama_host(&app);
    let url = format!("{}/api/pull", host.trim_end_matches('/'));
    let mut response = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({ "model": model, "stream": true }))
        .send()
        .await
        .map_err(|e| format!("Couldn't reach Ollama at {host}: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Ollama refused to pull '{model}' ({status}): {body}"));
    }

    let mut buffer = String::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("The download stopped: {e}"))?
    {
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(newline) = buffer.find('\n') {
            let line: String = buffer.drain(..=newline).collect();
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if let Some(error) = value.get("error").and_then(|e| e.as_str()) {
                return Err(format!("Ollama couldn't pull '{model}': {error}"));
            }
            let _ = app.emit(
                "ollama-pull-progress",
                serde_json::json!({
                    "status": value.get("status").and_then(|s| s.as_str()).unwrap_or(""),
                    "completed": value.get("completed").and_then(|c| c.as_u64()).unwrap_or(0),
                    "total": value.get("total").and_then(|t| t.as_u64()).unwrap_or(0),
                }),
            );
        }
    }

    Ok(())
}

/// OCR for the paths that only want text, whichever engine provides it.
///
/// The OS engine first because it needs no install - the Ollama method
/// used to demand a Tesseract install on machines that already had a
/// working engine the rest of the app was using. Tesseract stays as the
/// fallback, and its error is the one reported when neither works, since
/// on a machine with no OS engine that is the one the user can fix.
async fn read_with_best_engine(bytes: &[u8]) -> Result<Vec<crate::local_ocr::OcrLine>, String> {
    if crate::system_ocr::available() {
        match crate::system_ocr::run(bytes).await {
            Ok(lines) => return Ok(lines),
            // Falling through rather than failing: an engine that is
            // present can still refuse a particular image, and Tesseract
            // may well read it.
            Err(_) => {}
        }
    }
    crate::local_ocr::run_ocr(bytes)
}

/// Reads the screenshot with whichever OCR engine this machine has, then
/// has a local model pick the fields out of the text.
///
/// Returns the OCR blocks like the plain Tesseract path does, so
/// click-to-fill and learning from corrections work here too - the text was
/// read the same way either way.
#[tauri::command]
pub async fn extract_with_ollama<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    image_base64: String,
) -> Result<LocalOcrResult, String> {
    let host = resolve_ollama_host(&app);
    let preferred = resolve_model(&app, "ollama");
    let unload = resolve_unload_after_use(&app);
    let installed = installed_models(&host).await;

    // A vision model reads the screenshot itself, so Tesseract is skipped
    // entirely and no OCR mistake can be inherited. Decided before the OCR
    // run rather than after, so nothing is done twice.
    let vision_choice = installed
        .as_ref()
        .and_then(|models| crate::models::best_available_model(&preferred, models))
        .filter(|m| crate::models::is_vision_model(m));
    if let Some(model) = vision_choice {
        let result =
            extraction::extract_fields_with_ollama_vision(&host, &model, &image_base64, unload)
                .await?;
        return Ok(LocalOcrResult {
            result,
            // A vision model produces no text blocks, so click-to-fill has
            // nothing to offer - the fields came from the image itself.
            blocks: Vec::new(),
            site: None,
        });
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64)
        .map_err(|e| format!("Invalid image data: {e}"))?;

    let lines = read_with_best_engine(&bytes).await?;
    if lines.is_empty() {
        return Ok(LocalOcrResult {
            result: ExtractionResult::ParseFailed {
                raw_text: String::new(),
                error: "No text was detected in the image.".to_string(),
            },
            blocks: Vec::new(),
            site: None,
        });
    }

    let blocks: Vec<String> = lines.iter().map(|l| l.text.trim().to_string()).collect();
    let site = crate::local_ocr::detect_site(&blocks.join("\n"));

    // Use whatever is actually pulled rather than failing on a model the
    // user never asked for and does not have.
    let model = match installed {
        Some(models) => crate::models::best_available_model(&preferred, &models)
            .ok_or_else(|| {
                format!(
                    "Ollama is running but has no usable model. Pull one first: `ollama pull {}`",
                    crate::models::provider_or_default("ollama").default_model
                )
            })?,
        None => preferred,
    };
    let result = extraction::extract_fields_from_text(&host, &model, &blocks.join("\n"), unload)
            .await?;

    Ok(LocalOcrResult {
        result,
        blocks,
        site: site.map(str::to_string),
    })
}

/// The models a stored key can actually reach.
///
/// Only providers speaking OpenAI's wire format have a listing endpoint.
/// Returns an empty list rather than an error when there is no key or no
/// endpoint - the Model field falls back to free text, which is what it
/// was before.
#[tauri::command]
pub async fn list_provider_models(provider: String) -> Vec<String> {
    let Some(p) = crate::models::find_provider(&provider) else {
        return Vec::new();
    };
    let Ok(key) = keychain::get_api_key(&p.id) else {
        return Vec::new();
    };
    extraction::list_models(&p.id, &key, &p.api_base).await
}

/// Chooses a model for a provider when the configured one is not among
/// those its key can reach.
///
/// The shipped default is a guess made before anyone had a key. A
/// university gateway serves an entirely different set - "protected.Claude
/// Sonnet 4.6" rather than "gpt-4o" - so the guess is usually wrong there
/// and fails as a 404 at the moment of capture. Returns the model now in
/// force, and whether it had to change.
#[tauri::command]
pub async fn auto_select_model<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    provider: String,
) -> Result<ModelChoice, String> {
    let current = resolve_model(&app, &provider);
    let available = list_provider_models(provider.clone()).await;

    // No listing endpoint, or no key yet: keep what is configured.
    if available.is_empty() {
        return Ok(ModelChoice {
            model: current,
            changed: false,
            available: Vec::new(),
        });
    }
    if available.iter().any(|m| m == &current) {
        return Ok(ModelChoice {
            model: current,
            changed: false,
            available,
        });
    }

    let Some(best) = crate::models::best_cloud_model(&available) else {
        return Ok(ModelChoice {
            model: current,
            changed: false,
            available,
        });
    };
    set_model(app, provider, best.clone())?;
    Ok(ModelChoice {
        model: best,
        changed: true,
        available,
    })
}

#[derive(serde::Serialize)]
pub struct ModelChoice {
    pub model: String,
    /// True when the configured model was not on offer and was replaced.
    pub changed: bool,
    pub available: Vec<String>,
}

/// Checks a stored key against the provider, without a screenshot.
///
/// A failed capture cannot tell you whether the key is wrong, the model is
/// gone, or the service is busy - all three arrive as one line at the
/// moment you were trying to do something else. This asks the provider
/// directly, and for the OpenAI-compatible ones it lists the models the
/// key can actually reach, which is the answer to "what do I put in the
/// Model field".
#[tauri::command]
pub async fn test_api_key(provider: String) -> Result<String, String> {
    let p = crate::models::find_provider(&provider)
        .ok_or_else(|| format!("'{provider}' is not a method this build knows."))?;
    if !p.needs_key {
        return Ok(format!("{} needs no key.", p.label));
    }
    let key = keychain::get_api_key(&p.id)
        .map_err(|_| format!("No key stored for {} yet.", p.label))?;

    let names = extraction::list_models(&p.id, &key, &p.api_base).await;
    if names.is_empty() {
        return Err(format!(
            "{} did not accept the key, or returned no usable models.",
            p.label
        ));
    }
    Ok(format!(
        "Key works. {} models available, including: {}.",
        names.len(),
        names.iter().take(6).cloned().collect::<Vec<_>>().join(", ")
    ))
}

/// The OCR blocks travel to the frontend alongside the guessed fields:
/// they are what the click-to-fill list is built from, and what a later
/// correction is matched against.
#[derive(serde::Serialize)]
pub struct LocalOcrResult {
    #[serde(flatten)]
    pub result: ExtractionResult,
    pub blocks: Vec<String>,
    pub site: Option<String>,
}

fn read_hints<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    site: &str,
) -> crate::local_ocr::FieldHints {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(OCR_HINTS_KEY))
        .and_then(|v| v.get(site).cloned())
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

/// Records where the values the user kept actually sat on the page.
///
/// Called after a successful save, with the fields as saved rather than as
/// guessed, so a correction teaches the next capture from the same board.
/// Best-effort: never fails a save.
#[tauri::command]
pub fn learn_ocr_hints<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    site: String,
    blocks: Vec<String>,
    saved: Vec<(String, String)>,
) -> Result<(), String> {
    if site.is_empty() || blocks.is_empty() {
        return Ok(());
    }
    let learned = crate::local_ocr::learn_hints(&blocks, &saved);
    if learned.is_empty() {
        return Ok(());
    }

    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    let mut all = store
        .get(OCR_HINTS_KEY)
        .and_then(|v| {
            serde_json::from_value::<
                std::collections::HashMap<String, crate::local_ocr::FieldHints>,
            >(v)
            .ok()
        })
        .unwrap_or_default();

    // Merge rather than replace: a capture that only pinned down the
    // company should not forget where the title was.
    all.entry(site).or_default().extend(learned);

    store.set(
        OCR_HINTS_KEY,
        serde_json::to_value(&all).map_err(|e| e.to_string())?,
    );
    store
        .save()
        .map_err(|e| format!("Could not persist settings: {e}"))
}

#[tauri::command]
pub fn get_excel_path<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    let path = resolve_excel_path(&app)?;
    Ok(path.to_string_lossy().to_string())
}

/// What happened when the workbook path changed.
#[derive(Debug, serde::Serialize, PartialEq)]
#[serde(tag = "outcome")]
pub enum PathChange {
    /// The path was pointed somewhere else, nothing was moved.
    Switched,
    Moved {
        backups: usize,
        screenshots: usize,
    },
    /// Something is already at the destination, so nothing was moved: the
    /// path still changed, and both files are left intact.
    DestinationExists,
}

#[tauri::command]
pub fn workbook_exists<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> bool {
    resolve_excel_path(&app).map(|p| p.is_file()).unwrap_or(false)
}

/// Moves a file, then removes the original.
///
/// Copy-then-remove rather than rename: a rename fails across volumes, and
/// picking a folder on another drive is exactly what someone does when they
/// move a workbook to a USB stick or a synced folder. Removing only after a
/// successful copy means a failure loses nothing.
fn move_file(from: &std::path::Path, to: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create '{}': {e}", parent.display()))?;
    }
    std::fs::copy(from, to)
        .map_err(|e| format!("Could not copy to '{}': {e}", to.display()))?;
    std::fs::remove_file(from)
        .map_err(|e| format!("Copied to '{}', but could not remove the original: {e}", to.display()))
}

/// Points the app at a different workbook, optionally taking the existing
/// one with it.
///
/// The screenshots folder has to come along or every archived capture stops
/// being findable, since it is located relative to the workbook. Backups
/// follow too, but only the ones belonging to this workbook - another
/// workbook may share the folder.
#[tauri::command]
pub fn set_excel_path<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: String,
    move_existing: bool,
) -> Result<PathChange, String> {
    let new = PathBuf::from(&path);

    // Only resolved when a move is actually wanted, and never fatal.
    // Setting a path must work even when the old one cannot be worked out -
    // if the Documents folder is unavailable, choosing a location by hand is
    // exactly the fix, and failing here would take that away.
    let old = if move_existing {
        resolve_excel_path(&app).ok()
    } else {
        None
    };

    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;

    let mut outcome = PathChange::Switched;

    if let Some(old) = old.filter(|o| o.is_file() && *o != new) {
        if new.exists() {
            // Never clobber a workbook that is already there - it may be
            // the one being switched to on purpose.
            outcome = PathChange::DestinationExists;
        } else {
            move_file(&old, &new)?;

            let stem = old
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("JobApplications")
                .to_string();
            let backups = move_matching_backups(&old, &new, &stem);
            let screenshots = move_screenshots(&old, &new);
            outcome = PathChange::Moved {
                backups,
                screenshots,
            };
        }
    }

    store.set(EXCEL_PATH_KEY, serde_json::json!(path));
    store
        .save()
        .map_err(|e| format!("Could not persist settings: {e}"))?;
    Ok(outcome)
}

/// Best-effort: a workbook that moved without its backups is still usable,
/// so a failure here must not fail the move.
fn move_matching_backups(old: &std::path::Path, new: &std::path::Path, stem: &str) -> usize {
    let (Some(old_dir), Some(new_dir)) = (old.parent(), new.parent()) else {
        return 0;
    };
    let from = old_dir.join("backups");
    let to = new_dir.join("backups");
    let Ok(entries) = std::fs::read_dir(&from) else {
        return 0;
    };
    let prefix = format!("{stem}_");
    let mut moved = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else { continue };
        // Only this workbook's backups; another may share the folder.
        if !name_str.starts_with(&prefix) {
            continue;
        }
        if move_file(&entry.path(), &to.join(name_str)).is_ok() {
            moved += 1;
        }
    }
    let _ = std::fs::remove_dir(&from); // only succeeds when now empty
    moved
}

fn move_screenshots(old: &std::path::Path, new: &std::path::Path) -> usize {
    let (Some(old_dir), Some(new_dir)) = (old.parent(), new.parent()) else {
        return 0;
    };
    let from = old_dir.join(SCREENSHOTS_DIR);
    let to = new_dir.join(SCREENSHOTS_DIR);
    let Ok(entries) = std::fs::read_dir(&from) else {
        return 0;
    };
    let mut moved = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name_str) = name.to_str() else { continue };
        let target = to.join(name_str);
        // Never overwrite a capture already at the destination.
        if target.exists() {
            continue;
        }
        if move_file(&entry.path(), &target).is_ok() {
            moved += 1;
        }
    }
    let _ = std::fs::remove_dir(&from);
    moved
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
    excel::write_applications(&path, &rows, &status_names(&app))?;
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

    excel::write_applications(&path, &rows, &status_names(&app))
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

    excel::write_applications(&path, &rows, &status_names(&app))
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

    excel::write_applications(&path, &rows, &status_names(&app))
}

/// Removes a row.
///
/// The company and position the user was looking at are passed back in and
/// checked against the row actually at that index. The workbook is a plain
/// file the user can edit in Excel while this app is open, so an index alone
/// is not enough to be sure the right row is being destroyed - and unlike
/// every other write here, this one has nothing to undo it from inside the
/// app. (`write_applications` still takes its usual timestamped backup, so a
/// wrong delete is recoverable from `backups/`.)
#[tauri::command]
pub fn delete_application_at_index<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    index: usize,
    expected_company: String,
    expected_position: String,
) -> Result<(), String> {
    let path = resolve_excel_path(&app)?;
    let mut rows = excel::read_applications(&path)?;

    let row = rows
        .get(index)
        .ok_or_else(|| "That row no longer exists - reload the list and try again.".to_string())?;

    let expected = (
        expected_company.trim().to_lowercase(),
        expected_position.trim().to_lowercase(),
    );
    if row.dedupe_key() != expected {
        return Err(format!(
            "That row is now '{} - {}', not the one you asked to delete. The workbook changed underneath - reload the list and try again.",
            row.company, row.position
        ));
    }

    rows.remove(index);
    excel::write_applications(&path, &rows, &status_names(&app))
}

/// Puts a row back where it was, undoing a delete.
///
/// The index is clamped rather than rejected: by the time someone clicks
/// Undo the workbook may legitimately have fewer rows than it did (another
/// delete, an edit in Excel), and refusing to restore their application
/// because its old position no longer exists would be the wrong call. Worst
/// case it lands at the end, which the user can see and re-sort.
#[tauri::command]
pub fn insert_application_at_index<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    index: usize,
    application: JobApplication,
) -> Result<(), String> {
    let path = resolve_excel_path(&app)?;
    let mut rows = excel::read_applications(&path)?;

    let at = index.min(rows.len());
    rows.insert(at, application);
    excel::write_applications(&path, &rows, &status_names(&app))
}

/// Merges the rows of another `.xlsx` into the current workbook.
///
/// Anything whose company+position already exists here is skipped rather
/// than duplicated, and rows with neither a company nor a position (blank
/// or junk lines) are counted out. Nothing in the source file is modified.
#[tauri::command]
pub fn import_applications<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: String,
) -> Result<ImportSummary, String> {
    let source = PathBuf::from(&path);
    if !source.exists() {
        return Err(format!("'{path}' no longer exists."));
    }
    let target_path = resolve_excel_path(&app)?;
    if source == target_path {
        return Err("That is the workbook you are already tracking in.".to_string());
    }

    let incoming = excel::read_applications(&source)?;
    let mut rows = excel::read_applications(&target_path)?;

    let mut existing: std::collections::HashSet<(String, String)> =
        rows.iter().map(|r| r.dedupe_key()).collect();

    let mut summary = ImportSummary::default();
    for mut candidate in incoming {
        if candidate.company.trim().is_empty() && candidate.position.trim().is_empty() {
            summary.skipped_blank += 1;
            continue;
        }
        if !existing.insert(candidate.dedupe_key()) {
            summary.skipped_duplicates += 1;
            continue;
        }
        if candidate.status.trim().is_empty() {
            candidate.status = "Applied".to_string();
        }
        rows.push(candidate);
        summary.imported += 1;
    }

    if summary.imported > 0 {
        excel::write_applications(&target_path, &rows, &status_names(&app))?;
    }
    Ok(summary)
}

#[derive(Debug, Default, serde::Serialize)]
pub struct ImportSummary {
    pub imported: usize,
    pub skipped_duplicates: usize,
    pub skipped_blank: usize,
}

#[tauri::command]
pub async fn pick_import_file<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Excel Workbook", &["xlsx"])
        .pick_file(move |file_path| {
            let _ = tx.send(file_path);
        });
    let result = rx.recv().map_err(|e| format!("File dialog failed: {e}"))?;
    Ok(result.map(|p| p.to_string()))
}

#[tauri::command]
pub fn export_csv<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    let path = resolve_excel_path(&app)?;
    let rows = excel::read_applications(&path)?;
    let csv_path = excel::export_csv(&path, &rows)?;
    Ok(csv_path.to_string_lossy().to_string())
}

const SCREENSHOTS_DIR: &str = "JobApplications_screenshots";

/// Extensions `save_screenshot` can have written, newest-first by how
/// likely they are: the clipboard path always produces PNG.
const SCREENSHOT_EXTENSIONS: [&str; 4] = ["png", "jpg", "webp", "gif"];

/// The name `save_screenshot` gives a capture, without its extension.
/// Kept next to the writer so the two can't drift apart - a lookup that
/// builds the name differently would silently find nothing.
fn screenshot_stem(company: &str, position: &str, date_applied: &str) -> String {
    format!(
        "{}_{}_{}",
        sanitize_for_filename(date_applied),
        sanitize_for_filename(company),
        sanitize_for_filename(position)
    )
}

/// Finds the archived screenshot for a row, if one was ever saved. Returns
/// None rather than erroring: most rows have no screenshot (typed by hand,
/// imported, or captured before archiving existed), and that is not a fault.
pub fn find_screenshot(
    workbook: &std::path::Path,
    company: &str,
    position: &str,
    date_applied: &str,
) -> Option<PathBuf> {
    let dir = workbook
        .parent()
        .filter(|p| !p.as_os_str().is_empty())?
        .join(SCREENSHOTS_DIR);
    let stem = screenshot_stem(company, position, date_applied);
    SCREENSHOT_EXTENSIONS
        .iter()
        .map(|ext| dir.join(format!("{stem}.{ext}")))
        .find(|p| p.is_file())
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

    let screenshots_dir = parent.join(SCREENSHOTS_DIR);
    std::fs::create_dir_all(&screenshots_dir)
        .map_err(|e| format!("Could not create '{}': {e}", screenshots_dir.display()))?;

    let file_name = format!(
        "{}.{}",
        screenshot_stem(&company, &position, &date_applied),
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

/// Whether the app checks for a new release on startup. On by default;
/// the Settings tab toggles it.
#[tauri::command]
pub fn get_update_check_enabled<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<bool, String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    Ok(store
        .get(UPDATE_CHECK_ENABLED_KEY)
        .and_then(|v| v.as_bool())
        .unwrap_or(true))
}

#[tauri::command]
pub fn set_update_check_enabled<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    enabled: bool,
) -> Result<(), String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    store.set(UPDATE_CHECK_ENABLED_KEY, serde_json::json!(enabled));
    store
        .save()
        .map_err(|e| format!("Could not persist settings: {e}"))
}

/// Whether a found update installs itself, or waits for the banner's
/// button. On by default - the frontend still holds off while there's
/// unsaved work in the capture form.
#[tauri::command]
pub fn get_auto_install_updates<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<bool, String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    Ok(store
        .get(AUTO_INSTALL_UPDATES_KEY)
        .and_then(|v| v.as_bool())
        .unwrap_or(true))
}

#[tauri::command]
pub fn set_auto_install_updates<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    enabled: bool,
) -> Result<(), String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    store.set(AUTO_INSTALL_UPDATES_KEY, serde_json::json!(enabled));
    store
        .save()
        .map_err(|e| format!("Could not persist settings: {e}"))
}

#[tauri::command]
pub fn get_app_version<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> String {
    app.package_info().version.to_string()
}

/// Asks whether a newer release is out. Called once when the window
/// initialises (throttled to once a day, skipped if the user turned
/// automatic checks off) and again whenever "Check now" is clicked, which
/// passes `force` to bypass both.
#[tauri::command]
pub async fn check_for_update<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    force: bool,
) -> Result<UpdateCheck, String> {
    // Read the settings and drop the store handle before awaiting, so
    // nothing non-Send is held across the network call.
    let (enabled, last_check) = {
        let store = app
            .store(STORE_FILE)
            .map_err(|e| format!("Could not open settings store: {e}"))?;
        let enabled = store
            .get(UPDATE_CHECK_ENABLED_KEY)
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let last_check = store
            .get(LAST_UPDATE_CHECK_KEY)
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        (enabled, last_check)
    };

    let result = updates::check(&app, force, enabled, last_check).await?;

    // Only a check that actually went out resets the once-a-day clock.
    if !matches!(result, UpdateCheck::Skipped { .. }) {
        let store = app
            .store(STORE_FILE)
            .map_err(|e| format!("Could not open settings store: {e}"))?;
        store.set(
            LAST_UPDATE_CHECK_KEY,
            serde_json::json!(chrono::Utc::now().to_rfc3339()),
        );
        let _ = store.save();
    }

    Ok(result)
}

/// Downloads and installs the pending update, then restarts into it.
/// Only ever called from the update banner's button.
#[tauri::command]
pub async fn install_update<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<(), String> {
    updates::install(&app).await
}

/// Checks a row's URL before it is handed to the OS opener, returning the
/// normalized form.
///
/// A row's URL can come from Claude's reading of a screenshot, from
/// Tesseract's OCR, or from a hand-edited spreadsheet cell, so it is never
/// something the app itself put there. Anything but http/https - `file:`,
/// `javascript:`, a shell path - is refused rather than opened.
fn validated_http_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let parsed = url::Url::parse(trimmed)
        .map_err(|_| format!("'{trimmed}' isn't a URL this can open."))?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed.to_string()),
        other => Err(format!(
            "Refusing to open a '{other}' link - only http and https are allowed."
        )),
    }
}

/// Opens a saved application's URL in the user's real browser.
#[tauri::command]
pub fn open_url<R: tauri::Runtime>(app: tauri::AppHandle<R>, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let safe = validated_http_url(&url)?;
    app.opener()
        .open_url(safe, None::<&str>)
        .map_err(|e| format!("Couldn't open that link: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_run_default_is_an_engine_this_machine_can_actually_run() {
        let method = default_extraction_method();
        assert_eq!(
            method == "system",
            crate::system_ocr::available(),
            "a machine with an OS engine should start on it, one without should not"
        );
        // Whatever it picks has to be a real provider, or the Settings
        // dropdown opens with nothing selected.
        assert_eq!(crate::models::provider_or_default(&method).id, method);
        // And it must never be one that asks for a key before the user has
        // been anywhere near Settings.
        assert!(!crate::models::provider_or_default(&method).needs_key);
    }

    #[test]
    fn accepts_the_urls_a_job_posting_actually_has() {
        for url in [
            "https://www.linkedin.com/jobs/view/4123456789",
            "http://careers.example.com/posting?id=7",
            "https://example.com/a%20path#frag",
        ] {
            assert!(validated_http_url(url).is_ok(), "should accept {url}");
        }
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(
            validated_http_url("  https://example.com/  ").unwrap(),
            "https://example.com/"
        );
    }

    #[test]
    fn refuses_every_scheme_but_http_and_https() {
        for url in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:text/html,<script>alert(1)</script>",
            "ftp://example.com/x",
            "vbscript:msgbox(1)",
        ] {
            let err = validated_http_url(url).expect_err("should refuse {url}");
            assert!(err.contains("Refusing to open"), "unexpected error: {err}");
        }
    }

    #[test]
    fn refuses_things_that_are_not_urls_at_all() {
        for raw in ["", "   ", "not a url", r"C:\Windows\System32", "example.com"] {
            assert!(validated_http_url(raw).is_err(), "should refuse {raw:?}");
        }
    }
}

/// The shortcut that shows the capture window, as a Tauri accelerator
/// string such as "Ctrl+Shift+J". Falls back to the platform default.
#[tauri::command]
pub fn get_hotkey<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> String {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(HOTKEY_KEY))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| crate::default_hotkey().to_string())
}

/// Re-registers the global shortcut, keeping the old one if the new one
/// can't be taken (another app already owns it, or it isn't a valid
/// accelerator). Returns the shortcut actually in force.
#[tauri::command]
pub fn set_hotkey<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    shortcut: String,
) -> Result<String, String> {
    let previous = get_hotkey(app.clone());
    let wanted = shortcut.trim().to_string();
    if wanted.is_empty() {
        return Err("Pick a shortcut first.".to_string());
    }

    crate::register_capture_shortcut(&app, &wanted).map_err(|e| {
        // Put the working one back so the user is never left with no way in.
        let _ = crate::register_capture_shortcut(&app, &previous);
        format!("Couldn't use {wanted}: {e}")
    })?;

    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    store.set(HOTKEY_KEY, serde_json::json!(wanted));
    store
        .save()
        .map_err(|e| format!("Could not persist settings: {e}"))?;
    Ok(wanted)
}

/// Whether the welcome panel has been dismissed before. Used to show the
/// window on a genuine first run rather than leaving a new user staring at
/// a tray icon they have no reason to click.
#[tauri::command]
pub fn get_seen_welcome<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> bool {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(SEEN_WELCOME_KEY))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

#[tauri::command]
pub fn set_seen_welcome<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<(), String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    store.set(SEEN_WELCOME_KEY, serde_json::json!(true));
    store
        .save()
        .map_err(|e| format!("Could not persist settings: {e}"))
}

/// Whether this row has an archived screenshot, so the UI can offer to
/// open it only when there is one.
#[tauri::command]
pub fn screenshot_for_application<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    company: String,
    position: String,
    date_applied: String,
) -> Result<Option<String>, String> {
    let workbook = resolve_excel_path(&app)?;
    Ok(find_screenshot(&workbook, &company, &position, &date_applied)
        .map(|p| p.to_string_lossy().to_string()))
}

/// Opens an archived screenshot in the system image viewer.
///
/// The path is re-derived from the row rather than trusted from the
/// frontend: this hands a path to the OS opener, and the only paths it
/// should ever open are files this app wrote into its own screenshots
/// folder.
#[tauri::command]
pub fn open_screenshot<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    company: String,
    position: String,
    date_applied: String,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let workbook = resolve_excel_path(&app)?;
    let path = find_screenshot(&workbook, &company, &position, &date_applied)
        .ok_or_else(|| "No screenshot was archived for this application.".to_string())?;

    app.opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| format!("Couldn't open the screenshot: {e}"))
}

/// Methods to try, in order, when the chosen one fails.
///
/// Extraction fails for reasons that have nothing to do with the
/// screenshot: an expired key, a rate limit, a model retired by its
/// provider, no network. A second choice turns any of those from "type it
/// in by hand" into a pause.
#[tauri::command]
pub fn get_fallback_chain<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Vec<String> {
    effective_fallbacks(&app)
}

/// The stored list minus anything that cannot be a fallback now.
///
/// The saved list was filtered against the primary at the time it was
/// saved, which stops being true the moment the primary changes. Picking a
/// method already in the list otherwise left it in both places: it ran
/// twice per capture - two waits, two charges - and reported the same
/// failure twice. Filtered on the way out, so the stored order survives
/// switching the primary back and forth.
fn effective_fallbacks<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Vec<String> {
    let primary = get_extraction_method(app.clone()).unwrap_or_default();
    let mut seen = std::collections::HashSet::new();
    read_fallback_chain(app)
        .into_iter()
        .filter(|id| id != &primary)
        .filter(|id| crate::models::find_provider(id).is_some())
        .filter(|id| seen.insert(id.clone()))
        .collect()
}

fn read_fallback_chain<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Vec<String> {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(FALLBACK_CHAIN_KEY))
        .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn set_fallback_chain<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    chain: Vec<String>,
) -> Result<Vec<String>, String> {
    // Only methods this build knows, each at most once, and never the
    // primary - trying the thing that just failed again is not a fallback.
    let primary = get_extraction_method(app.clone())?;
    let mut seen = std::collections::HashSet::new();
    let cleaned: Vec<String> = chain
        .into_iter()
        .filter(|id| crate::models::find_provider(id).is_some())
        .filter(|id| id != &primary)
        .filter(|id| seen.insert(id.clone()))
        .collect();

    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("Could not open settings store: {e}"))?;
    store.set(
        FALLBACK_CHAIN_KEY,
        serde_json::to_value(&cleaned).map_err(|e| e.to_string())?,
    );
    store
        .save()
        .map_err(|e| format!("Could not persist settings: {e}"))?;
    Ok(cleaned)
}

/// The result of an extraction, plus which method actually produced it.
#[derive(serde::Serialize)]
pub struct ChainResult {
    #[serde(flatten)]
    pub outcome: LocalOcrResult,
    /// The provider that succeeded, so the UI can say what read the page.
    pub used: String,
    /// What was tried and failed first, for when the answer looks odd.
    pub fell_back_from: Vec<String>,
}

/// Trims an error to the part that says what went wrong.
///
/// Several of these carry a paragraph of installation advice, which is
/// useful on its own and unreadable stacked three deep.
fn first_sentence(message: &str) -> String {
    const LIMIT: usize = 160;
    let trimmed = message.trim();

    // Counted in characters throughout. `find` returns a byte offset, and
    // taking that many *characters* silently keeps too much the moment a
    // provider's message contains a curly quote or a dash - which they do.
    let sentence_end = trimmed
        .find(". ")
        .map(|byte_idx| trimmed[..=byte_idx].chars().count());
    let total = trimmed.chars().count();
    let cut = sentence_end.unwrap_or(total).min(LIMIT);

    if cut >= total {
        return trimmed.to_string();
    }

    let mut short: String = trimmed.chars().take(cut).collect();
    // A hard limit lands mid-word; back up to the last space so it reads
    // as a truncation rather than a typo.
    if sentence_end.map(|e| e > LIMIT).unwrap_or(true) {
        if let Some(space) = short.rfind(' ') {
            short.truncate(space);
        }
    }
    format!("{}...", short.trim_end_matches(['.', ',', ' ']))
}

#[cfg(test)]
pub fn first_sentence_for_test(message: &str) -> String {
    first_sentence(message)
}

/// Runs one method, whichever kind it is.
async fn run_one<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    provider: &str,
    image_base64: &str,
    media_type: &str,
) -> Result<LocalOcrResult, String> {
    match provider {
        "system" => extract_with_system_ocr(app.clone(), image_base64.to_string()).await,
        "tesseract" => extract_with_local_ocr(app.clone(), image_base64.to_string()),
        "ollama" => extract_with_ollama(app.clone(), image_base64.to_string()).await,
        _ => {
            // By name, so a fallback runs itself rather than the primary.
            let result = run_cloud(app, provider, image_base64, media_type).await?;
            // A cloud model reads the image itself, so there are no OCR
            // blocks to offer for click-to-fill.
            Ok(LocalOcrResult {
                result,
                blocks: Vec::new(),
                site: None,
            })
        }
    }
}

/// Tries the chosen method, then each fallback in turn, and returns the
/// first that works.
///
/// A result the user has to correct still counts as working - only an
/// outright failure moves on, since a provider that answered has done its
/// job and the next one is unlikely to do better on the same image.
#[tauri::command]
pub async fn extract_with_chain<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    image_base64: String,
    media_type: String,
) -> Result<ChainResult, String> {
    let primary = get_extraction_method(app.clone())?;
    let mut chain = vec![primary];
    chain.extend(effective_fallbacks(&app));

    let mut failed = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    // A method that answered with something unreadable is kept aside. Its
    // raw text is still worth showing if nothing else works, since it can
    // be corrected by hand - but it must not stop the chain, because the
    // next method may return proper fields for the same image.
    let mut unreadable: Option<(String, LocalOcrResult)> = None;

    for provider in &chain {
        match run_one(&app, provider, &image_base64, &media_type).await {
            Ok(outcome) if matches!(outcome.result, ExtractionResult::ParseFailed { .. }) => {
                let label = crate::models::find_provider(provider)
                    .map(|p| p.label)
                    .unwrap_or_else(|| provider.clone());
                errors.push(format!("{label}: answered, but not with usable fields"));
                failed.push(provider.clone());
                if unreadable.is_none() {
                    unreadable = Some((provider.clone(), outcome));
                }
            }
            Ok(outcome) => {
                return Ok(ChainResult {
                    outcome,
                    used: provider.clone(),
                    fell_back_from: failed,
                })
            }
            Err(e) => {
                let label = crate::models::find_provider(provider)
                    .map(|p| p.label)
                    .unwrap_or_else(|| provider.clone());
                errors.push(format!("{label}: {}", first_sentence(&e)));
                failed.push(provider.clone());
            }
        }
    }

    // Nothing produced fields. An unreadable answer still beats nothing:
    // it carries the raw text into the Notes field to be fixed by hand.
    if let Some((provider, outcome)) = unreadable {
        let others: Vec<String> = failed.into_iter().filter(|p| p != &provider).collect();
        return Ok(ChainResult {
            outcome,
            used: provider,
            fell_back_from: others,
        });
    }

    // Every method's own reason, not just the last one's. The last is
    // usually the least interesting - it is whatever was at the bottom of
    // the list, while the failure worth reading is the first choice's.
    Err(errors.join("\n"))
}

// ---------- Resume tailoring ----------

/// The master resume, or an empty string when there isn't one yet.
#[tauri::command]
pub fn get_master_resume<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    let workbook = resolve_excel_path(&app)?;
    let Some(path) = crate::resume::master_path(&workbook) else {
        return Ok(String::new());
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("Could not read '{}': {e}", path.display())),
    }
}

/// Picks a resume file to import from.
#[tauri::command]
pub async fn pick_resume_file<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("Resume", &["pdf", "docx", "txt", "md"])
        .pick_file(move |file_path| {
            let _ = tx.send(file_path);
        });
    let result = rx.recv().map_err(|e| format!("File dialog failed: {e}"))?;
    Ok(result.map(|p| p.to_string()))
}

/// Reads a resume out of a PDF, a Word file or plain text, and stores it
/// as the master.
#[tauri::command]
pub fn import_resume_file<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: String,
) -> Result<String, String> {
    let text = crate::resume::import_text(std::path::Path::new(&path))?;
    set_master_resume(app, text.clone())?;
    Ok(text)
}

/// Saves the master resume beside the workbook, as a plain file the user
/// can open and edit in anything.
#[tauri::command]
pub fn set_master_resume<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    text: String,
) -> Result<String, String> {
    let workbook = resolve_excel_path(&app)?;
    let path = crate::resume::master_path(&workbook)
        .ok_or_else(|| "The workbook has no folder to save alongside.".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create '{}': {e}", parent.display()))?;
    }
    std::fs::write(&path, text)
        .map_err(|e| format!("Could not save '{}': {e}", path.display()))?;
    Ok(path.to_string_lossy().to_string())
}

/// Rewrites the master resume for one posting.
///
/// Uses whichever model-backed method is configured. The OCR engines can
/// read a screenshot but not write, so they are refused with a message
/// saying what to pick instead rather than a confusing failure.
#[tauri::command]
pub async fn tailor_resume<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    company: String,
    position: String,
    location: String,
    notes: String,
    pasted: String,
) -> Result<crate::resume_render::Resume, String> {
    let master = get_master_resume(app.clone())?;
    if master.trim().is_empty() {
        return Err("Save your full resume first - there is nothing to tailor.".to_string());
    }

    let method = get_extraction_method(app.clone())?;
    let provider = crate::models::provider_or_default(&method);
    if provider.default_model.is_empty() {
        return Err(format!(
            "{} reads screenshots but cannot write. Pick Ollama or a cloud method in Settings.",
            provider.label
        ));
    }

    let api_key = if provider.needs_key {
        keychain::get_api_key(&provider.id)?
    } else {
        String::new()
    };
    let api_base = if provider.id == "ollama" {
        resolve_ollama_host(&app)
    } else {
        provider.api_base.clone()
    };
    let model = resolve_model(&app, &provider.id);

    let brief = crate::resume::job_brief(&company, &position, &location, &notes, &pasted);
    let user = format!(
        "Here is the job posting:\n\n{brief}\n\n---\n\nHere is the full resume to tailor:\n\n{master}"
    );

    let raw = extraction::chat_text(
        &provider.id,
        &model,
        &api_key,
        &api_base,
        crate::resume::SYSTEM_PROMPT,
        &user,
        Some(crate::resume_render::schema()),
    )
    .await?;

    // A model that ignored the schema and answered in prose is a failure
    // worth naming, not something to paste into a PDF as-is.
    serde_json::from_str::<crate::resume_render::Resume>(&extraction::strip_fences(&raw))
        .map_err(|e| format!("{} did not return a resume it could use: {e}", provider.label))
}

/// Writes a tailored resume into a Resumes folder beside the workbook,
/// named after the job it was written for.
#[tauri::command]
pub fn save_tailored_resume<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    company: String,
    position: String,
    resume: crate::resume_render::Resume,
) -> Result<SavedResume, String> {
    let workbook = resolve_excel_path(&app)?;
    let dir = crate::resume::output_dir(&workbook)
        .ok_or_else(|| "The workbook has no folder to save alongside.".to_string())?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Could not create '{}': {e}", dir.display()))?;

    let stem = crate::resume::output_name(&company, &position);

    // The PDF is what gets attached; the .tex is there for anyone who
    // wants to typeset it themselves.
    let pdf_path = dir.join(format!("{stem}.pdf"));
    let pdf = crate::resume_render::to_pdf(&resume)?;
    std::fs::write(&pdf_path, pdf)
        .map_err(|e| format!("Could not save '{}': {e}", pdf_path.display()))?;

    let tex_path = dir.join(format!("{stem}.tex"));
    std::fs::write(&tex_path, crate::resume_render::to_latex(&resume))
        .map_err(|e| format!("Could not save '{}': {e}", tex_path.display()))?;

    // Record it against the application, so the row answers "what did I
    // send them?" the way it already answers "what did the posting say?".
    let pdf = pdf_path.to_string_lossy().to_string();
    let linked = link_resume_to_row(&app, &workbook, &company, &position, &pdf).unwrap_or(false);

    Ok(SavedResume {
        pdf,
        tex: tex_path.to_string_lossy().to_string(),
        linked,
    })
}

/// Writes the resume path onto the matching row.
///
/// Best-effort: the files are already written, and failing the save
/// because the workbook was open in Excel would lose the resume for no
/// reason. Matched the same way duplicate detection is, so a row saved
/// with different capitalisation still matches.
fn link_resume_to_row<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    workbook: &std::path::Path,
    company: &str,
    position: &str,
    pdf: &str,
) -> Result<bool, String> {
    if company.trim().is_empty() && position.trim().is_empty() {
        return Ok(false);
    }
    let mut rows = excel::read_applications(workbook)?;
    let key = (
        company.trim().to_lowercase(),
        position.trim().to_lowercase(),
    );
    let Some(row) = rows.iter_mut().find(|r| r.dedupe_key() == key) else {
        return Ok(false);
    };
    row.resume = Some(pdf.to_string());
    excel::write_applications(workbook, &rows, &status_names(app))?;
    Ok(true)
}

#[derive(serde::Serialize)]
pub struct SavedResume {
    pub pdf: String,
    pub tex: String,
    /// False when no saved application matched - a resume tailored from a
    /// pasted posting has no row to attach itself to.
    pub linked: bool,
}

/// Opens a saved resume in whatever the system uses for Markdown.
#[tauri::command]
pub fn open_saved_resume<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    path: String,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    // Re-derived rather than trusted: this hands a path to the OS opener,
    // and the only ones it should open are files this app wrote.
    let workbook = resolve_excel_path(&app)?;
    let dir = crate::resume::output_dir(&workbook)
        .ok_or_else(|| "The workbook has no folder.".to_string())?;
    let requested = PathBuf::from(&path);
    if requested.parent() != Some(dir.as_path()) {
        return Err("That file is not one of the saved resumes.".to_string());
    }

    app.opener()
        .open_path(requested.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| format!("Couldn't open it: {e}"))
}
