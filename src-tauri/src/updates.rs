//! Automatic update checking, backed by `tauri-plugin-updater`.
//!
//! The frontend has no npm packages, so this goes through `#[tauri::command]`s
//! using the plugin's Rust API rather than its JS one - the same shape as every
//! other capability in this app.
//!
//! In keeping with the rest of the app there is no background polling and no
//! timer: a check happens once when the window first initialises, at most once
//! a day, and can be turned off entirely. Nothing is ever downloaded or
//! installed without the user clicking the button.

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

/// Placeholder shipped in `tauri.conf.json`. Until the repo owner generates a
/// signing keypair and replaces it, checking would fail with an opaque
/// signature error - so it's detected up front and reported plainly instead.
pub const PUBKEY_PLACEHOLDER: &str = "REPLACE_WITH_YOUR_TAURI_PUBLIC_KEY";

/// Minimum gap between automatic checks. Manual checks ignore it.
const CHECK_INTERVAL_HOURS: i64 = 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome")]
pub enum UpdateCheck {
    /// An update is published and ready to install.
    Available {
        version: String,
        current_version: String,
        notes: Option<String>,
        date: Option<String>,
    },
    /// Checked, and this is already the newest release.
    UpToDate,
    /// No check was made: automatic checks are off, one already ran within the
    /// last day, or updates were never configured for this build.
    Skipped { reason: String },
}

/// Emitted repeatedly while an update downloads, so the banner can show
/// progress instead of sitting on "Downloading..." - which matters more now
/// that an install can start without the user having clicked anything.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

pub const PROGRESS_EVENT: &str = "update-download-progress";
pub const INSTALLING_EVENT: &str = "update-installing";

fn updater_is_configured<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> bool {
    serde_json::to_value(&app.config().plugins)
        .ok()
        .and_then(|plugins| {
            plugins
                .get("updater")
                .and_then(|updater| updater.get("pubkey"))
                .and_then(|key| key.as_str())
                .map(|key| !key.trim().is_empty() && key != PUBKEY_PLACEHOLDER)
        })
        .unwrap_or(false)
}

/// True when the last automatic check is old enough (or never happened) that
/// another one is due.
pub fn check_is_due(last_check: Option<&str>, now: chrono::DateTime<chrono::Utc>) -> bool {
    let Some(last_check) = last_check else {
        return true;
    };
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(last_check) else {
        // An unreadable timestamp shouldn't wedge checking forever.
        return true;
    };
    now.signed_duration_since(parsed.with_timezone(&chrono::Utc))
        .num_hours()
        >= CHECK_INTERVAL_HOURS
}

/// Asks the update endpoint whether there's a newer release.
///
/// `force` is what the Settings tab's "Check now" button passes: it ignores
/// both the once-a-day throttle and the automatic-checks-off preference, since
/// the user asked for this one explicitly.
pub async fn check<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    force: bool,
    enabled: bool,
    last_check: Option<String>,
) -> Result<UpdateCheck, String> {
    if !updater_is_configured(app) {
        return Ok(UpdateCheck::Skipped {
            reason: "This build has no update signing key configured, so it can't check for updates.".to_string(),
        });
    }
    if !force {
        if !enabled {
            return Ok(UpdateCheck::Skipped {
                reason: "Automatic update checks are turned off.".to_string(),
            });
        }
        if !check_is_due(last_check.as_deref(), chrono::Utc::now()) {
            return Ok(UpdateCheck::Skipped {
                reason: "Already checked for updates today.".to_string(),
            });
        }
    }

    let updater = app
        .updater()
        .map_err(|e| format!("Could not reach the update service: {e}"))?;

    match updater.check().await {
        Ok(Some(update)) => Ok(UpdateCheck::Available {
            version: update.version.clone(),
            current_version: update.current_version.clone(),
            notes: update.body.clone(),
            date: update.date.map(|d| d.to_string()),
        }),
        Ok(None) => Ok(UpdateCheck::UpToDate),
        Err(e) => Err(format!("Couldn't check for updates: {e}")),
    }
}

/// Downloads the pending update, installs it, and restarts into it.
///
/// The check is repeated rather than holding the `Update` handle across two
/// commands - it's one extra request, and it means a stale banner can never
/// install something the user didn't just see described.
pub async fn install<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<(), String> {
    let updater = app
        .updater()
        .map_err(|e| format!("Could not reach the update service: {e}"))?;

    let update = updater
        .check()
        .await
        .map_err(|e| format!("Couldn't check for updates: {e}"))?
        .ok_or_else(|| "There's no update to install - you're on the latest version.".to_string())?;

    let progress_handle = app.clone();
    let finished_handle = app.clone();
    let mut downloaded: u64 = 0;

    update
        .download_and_install(
            move |chunk_len, total| {
                downloaded += chunk_len as u64;
                let _ = progress_handle.emit(
                    PROGRESS_EVENT,
                    DownloadProgress {
                        downloaded,
                        total,
                    },
                );
            },
            move || {
                let _ = finished_handle.emit(INSTALLING_EVENT, ());
            },
        )
        .await
        .map_err(|e| format!("Couldn't install the update: {e}"))?;

    // Diverges - on Windows the installer usually takes the process down
    // itself before this is reached, so this is mainly the macOS path.
    app.restart()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn a_never_checked_app_is_due() {
        assert!(check_is_due(None, Utc::now()));
    }

    #[test]
    fn a_check_from_today_is_not_due_again() {
        let now = Utc::now();
        let an_hour_ago = (now - Duration::hours(1)).to_rfc3339();
        assert!(!check_is_due(Some(&an_hour_ago), now));
    }

    #[test]
    fn a_check_from_yesterday_is_due() {
        let now = Utc::now();
        let yesterday = (now - Duration::hours(25)).to_rfc3339();
        assert!(check_is_due(Some(&yesterday), now));
    }

    #[test]
    fn the_boundary_is_inclusive() {
        let now = Utc::now();
        let exactly_a_day = (now - Duration::hours(24)).to_rfc3339();
        assert!(check_is_due(Some(&exactly_a_day), now));
    }

    #[test]
    fn an_unreadable_timestamp_does_not_wedge_checking() {
        assert!(check_is_due(Some("not a timestamp"), Utc::now()));
        assert!(check_is_due(Some(""), Utc::now()));
    }
}
