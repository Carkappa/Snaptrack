use crate::models::JobApplication;
use calamine::{open_workbook, Data, Reader, Xlsx};
use rust_xlsxwriter::{Color, DataValidation, Format, FormatAlign, Workbook};
use std::path::{Path, PathBuf};

pub const SHEET_NAME: &str = "Applications";

pub const HEADERS: [&str; 12] = [
    "Date Applied",
    "Company",
    "Position",
    "Location",
    "Work Type",
    "Employment Type",
    "Salary Range",
    "Status",
    "Last Updated",
    "Job ID",
    "URL",
    "Notes",
];

/// Extra blank rows below existing data that still get the Status
/// dropdown validation, so rows added by hand in Excel stay constrained.
const VALIDATION_BUFFER_ROWS: u32 = 500;

fn cell_to_string(cell: Option<&Data>) -> String {
    match cell {
        Some(Data::String(s)) => s.clone(),
        Some(Data::Float(f)) => f.to_string(),
        Some(Data::Int(i)) => i.to_string(),
        Some(Data::Bool(b)) => b.to_string(),
        Some(Data::DateTime(dt)) => dt.to_string(),
        _ => String::new(),
    }
}

/// Excel stores a real date as a day count from its epoch, so a cell the
/// user formatted as a Date in Excel comes back as a bare number rather than
/// anything readable. Everything downstream - the Date column, the calendar,
/// the CSV - expects `YYYY-MM-DD`, so date columns go through here.
///
/// The epoch is 1899-12-30 rather than 12-31: Excel treats 1900 as a leap
/// year, and starting a day early absorbs that phantom day for every date
/// after 1900-03-01, which is every date an application can carry.
fn excel_serial_to_iso(serial: f64) -> Option<String> {
    let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?;
    let date = base.checked_add_signed(chrono::Duration::try_days(serial.trunc() as i64)?)?;
    Some(date.format("%Y-%m-%d").to_string())
}

/// Like `cell_to_string`, but for the two date columns: a genuinely
/// date-typed cell is rendered as `YYYY-MM-DD` instead of a serial number.
/// Anything already stored as text (which is what this app itself writes) is
/// passed through untouched.
fn cell_to_date_string(cell: Option<&Data>) -> String {
    match cell {
        Some(Data::DateTime(dt)) => {
            excel_serial_to_iso(dt.as_f64()).unwrap_or_else(|| cell_to_string(cell))
        }
        // Some producers write an ISO timestamp instead; keep the date half.
        Some(Data::DateTimeIso(s)) => s.split('T').next().unwrap_or(s).trim().to_string(),
        _ => cell_to_string(cell),
    }
}

fn cell_to_option(cell: Option<&Data>) -> Option<String> {
    let s = cell_to_string(cell);
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Reads all rows from the workbook. Returns an empty list if the file
/// doesn't exist yet (a brand-new tracker with nothing saved).
pub fn read_applications(path: &Path) -> Result<Vec<JobApplication>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut workbook: Xlsx<_> = open_workbook(path).map_err(|e| describe_open_error(e, path))?;

    // Fall back to the first sheet in case the file was hand-renamed.
    let sheet_to_read = if workbook.sheet_names().iter().any(|n| n == SHEET_NAME) {
        SHEET_NAME.to_string()
    } else {
        workbook
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| "Workbook has no sheets.".to_string())?
    };

    let range = workbook
        .worksheet_range(&sheet_to_read)
        .map_err(|e| format!("Could not read the '{sheet_to_read}' sheet: {e}"))?;

    let mut rows = Vec::new();
    for row in range.rows().skip(1) {
        // Skip fully blank rows (e.g. trailing formatted-but-empty rows).
        if row.iter().all(|c| cell_to_string(Some(c)).trim().is_empty()) {
            continue;
        }
        let app = JobApplication {
            date_applied: cell_to_date_string(row.first()),
            company: cell_to_string(row.get(1)),
            position: cell_to_string(row.get(2)),
            location: cell_to_option(row.get(3)),
            work_type: cell_to_option(row.get(4)),
            employment_type: cell_to_option(row.get(5)),
            salary_range: cell_to_option(row.get(6)),
            status: {
                let s = cell_to_string(row.get(7));
                if s.trim().is_empty() {
                    "Applied".to_string()
                } else {
                    s
                }
            },
            last_updated: cell_to_date_string(row.get(8)),
            job_id: cell_to_option(row.get(9)),
            url: cell_to_option(row.get(10)),
            notes: cell_to_option(row.get(11)),
        };
        if app.company.is_empty() && app.position.is_empty() {
            continue;
        }
        rows.push(app);
    }

    Ok(rows)
}

/// Exports the current rows to a `.csv` file next to the workbook
/// (same name, `.csv` extension), overwriting it each time.
pub fn export_csv(xlsx_path: &Path, rows: &[JobApplication]) -> Result<PathBuf, String> {
    let csv_path = xlsx_path.with_extension("csv");
    let mut writer = csv::Writer::from_path(&csv_path)
        .map_err(|e| format!("Could not create '{}': {e}", csv_path.display()))?;

    writer
        .write_record(HEADERS)
        .map_err(|e| format!("Could not write CSV header: {e}"))?;

    for app in rows {
        writer
            .write_record([
                app.date_applied.as_str(),
                app.company.as_str(),
                app.position.as_str(),
                app.location.as_deref().unwrap_or(""),
                app.work_type.as_deref().unwrap_or(""),
                app.employment_type.as_deref().unwrap_or(""),
                app.salary_range.as_deref().unwrap_or(""),
                app.status.as_str(),
                app.last_updated.as_str(),
                app.job_id.as_deref().unwrap_or(""),
                app.url.as_deref().unwrap_or(""),
                app.notes.as_deref().unwrap_or(""),
            ])
            .map_err(|e| format!("Could not write CSV row: {e}"))?;
    }

    writer
        .flush()
        .map_err(|e| format!("Could not finish writing '{}': {e}", csv_path.display()))?;

    Ok(csv_path)
}

fn describe_open_error(e: calamine::XlsxError, path: &Path) -> String {
    format!(
        "Could not open '{}'. It may be corrupted or not a valid .xlsx file: {e}",
        path.display()
    )
}

fn status_fill(status: &str) -> Option<Color> {
    match status {
        "Offered" => Some(Color::RGB(0xC6_EF_CE)),
        "Interviewing" => Some(Color::RGB(0xFF_EB_9C)),
        "Rejected" => Some(Color::RGB(0xFF_C7_CE)),
        "Ghosted" => Some(Color::RGB(0xD9_D9_D9)),
        _ => None,
    }
}

/// Rewrites the whole sheet from `rows`. Writes to a temp file in the
/// same directory first, then atomically renames it into place so a
/// crash mid-write can never corrupt the existing workbook. If the
/// target path is locked (e.g. open in Excel), the temp file is cleaned
/// up and a clear, retry-able error is returned - `rows` is never
/// consumed by this function's caller until it returns Ok, so the
/// caller can simply retry the same save.
pub fn write_applications(path: &Path, rows: &[JobApplication]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create folder '{}': {e}", parent.display()))?;
        }
    }

    let mut workbook = Workbook::new();
    let worksheet = workbook
        .add_worksheet()
        .set_name(SHEET_NAME)
        .map_err(|e| format!("Could not create sheet: {e}"))?;

    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xD9_E1_F2))
        .set_align(FormatAlign::Center);

    for (col, header) in HEADERS.iter().enumerate() {
        worksheet
            .write_string_with_format(0, col as u16, *header, &header_format)
            .map_err(|e| format!("Could not write header: {e}"))?;
    }

    for (i, app) in rows.iter().enumerate() {
        let row = (i + 1) as u32;
        let fill = status_fill(&app.status);
        let base_format = || {
            let f = Format::new();
            match fill {
                Some(color) => f.set_background_color(color),
                None => f,
            }
        };

        worksheet
            .write_string_with_format(row, 0, &app.date_applied, &base_format())
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(row, 1, &app.company, &base_format())
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(row, 2, &app.position, &base_format())
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(row, 3, app.location.as_deref().unwrap_or(""), &base_format())
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(row, 4, app.work_type.as_deref().unwrap_or(""), &base_format())
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(
                row,
                5,
                app.employment_type.as_deref().unwrap_or(""),
                &base_format(),
            )
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(
                row,
                6,
                app.salary_range.as_deref().unwrap_or(""),
                &base_format(),
            )
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(row, 7, &app.status, &base_format())
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(row, 8, &app.last_updated, &base_format())
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string_with_format(row, 9, app.job_id.as_deref().unwrap_or(""), &base_format())
            .map_err(|e| e.to_string())?;

        match app.url.as_deref() {
            Some(url) if !url.trim().is_empty() => {
                let hyperlink_format = {
                    let f = Format::new().set_font_color(Color::RGB(0x10_57_C4)).set_underline(rust_xlsxwriter::FormatUnderline::Single);
                    match fill {
                        Some(color) => f.set_background_color(color),
                        None => f,
                    }
                };
                let url_value = rust_xlsxwriter::Url::new(url);
                worksheet
                    .write_url_with_format(row, 10, url_value, &hyperlink_format)
                    .map_err(|e| format!("Could not write URL for row {row}: {e}"))?;
            }
            _ => {
                worksheet
                    .write_string_with_format(row, 10, "", &base_format())
                    .map_err(|e| e.to_string())?;
            }
        }

        worksheet
            .write_string_with_format(row, 11, app.notes.as_deref().unwrap_or(""), &base_format())
            .map_err(|e| e.to_string())?;
    }

    worksheet
        .set_freeze_panes(1, 0)
        .map_err(|e| format!("Could not freeze header row: {e}"))?;

    worksheet.autofit();

    let last_validation_row = (rows.len() as u32) + VALIDATION_BUFFER_ROWS;
    let status_validation = DataValidation::new()
        .allow_list_strings(&crate::models::STATUSES)
        .map_err(|e| format!("Could not build status dropdown: {e}"))?;
    worksheet
        .add_data_validation(1, 7, last_validation_row, 7, &status_validation)
        .map_err(|e| format!("Could not attach status dropdown: {e}"))?;

    let tmp_path = temp_path_for(path);
    workbook
        .save(&tmp_path)
        .map_err(|e| format!("Could not write workbook: {e}"))?;

    if path.exists() {
        // Best-effort: a failed backup should never block the actual save.
        let _ = backup_before_overwrite(path);
    }

    match std::fs::rename(&tmp_path, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path);
            if is_lock_error(&e) {
                Err(format!(
                    "'{}' is open in Excel (or another program) and can't be updated right now. Close it and click Retry.",
                    path.display()
                ))
            } else {
                Err(format!("Could not save to '{}': {e}", path.display()))
            }
        }
    }
}

const MAX_BACKUPS: usize = 10;

/// Copies the existing workbook into a `backups/` folder next to it,
/// timestamped, before it gets overwritten - cheap insurance against a
/// bad edit. Runs only at save time, so it stays consistent with the
/// app's "no background process" design.
fn backup_before_overwrite(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| "Workbook has no parent directory.".to_string())?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("JobApplications");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("xlsx");

    let backups_dir = parent.join("backups");
    std::fs::create_dir_all(&backups_dir).map_err(|e| e.to_string())?;

    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let backup_path = backups_dir.join(format!("{stem}_{timestamp}.{ext}"));
    std::fs::copy(path, &backup_path).map_err(|e| e.to_string())?;

    prune_old_backups(&backups_dir, stem, ext)
}

fn prune_old_backups(backups_dir: &Path, stem: &str, ext: &str) -> Result<(), String> {
    let prefix = format!("{stem}_");
    let suffix = format!(".{ext}");

    let mut backups: Vec<PathBuf> = std::fs::read_dir(backups_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(&prefix) && n.ends_with(&suffix))
                .unwrap_or(false)
        })
        .collect();

    // Timestamped filenames sort lexicographically in chronological order.
    backups.sort();

    if backups.len() > MAX_BACKUPS {
        for old in &backups[..backups.len() - MAX_BACKUPS] {
            let _ = std::fs::remove_file(old);
        }
    }

    Ok(())
}

fn is_lock_error(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::WouldBlock
    ) || e.raw_os_error() == Some(13) // EACCES
        || e.raw_os_error() == Some(32) // Windows ERROR_SHARING_VIOLATION mapped by std
}

fn temp_path_for(path: &Path) -> PathBuf {
    let file_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("JobApplications");
    let mut tmp = path.to_path_buf();
    tmp.set_file_name(format!(".{file_name}.tmp.xlsx"));
    tmp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::JobApplication;

    fn sample_app(company: &str, position: &str, status: &str) -> JobApplication {
        JobApplication {
            date_applied: "2026-09-01".to_string(),
            company: company.to_string(),
            position: position.to_string(),
            location: Some("Remote".to_string()),
            work_type: Some("Remote".to_string()),
            employment_type: Some("Full-time".to_string()),
            salary_range: None,
            status: status.to_string(),
            last_updated: "2026-09-01".to_string(),
            job_id: None,
            url: Some("https://example.com/job/123".to_string()),
            notes: None,
        }
    }

    #[test]
    fn write_then_read_roundtrips_rows() {
        let dir = std::env::temp_dir().join(format!("job-tracker-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("roundtrip.xlsx");

        let rows = vec![
            sample_app("Acme", "Engineer", "Applied"),
            sample_app("Globex", "Designer", "Interviewing"),
        ];
        write_applications(&path, &rows).expect("write should succeed");

        let read_back = read_applications(&path).expect("read should succeed");
        assert_eq!(read_back.len(), 2);
        assert_eq!(read_back[0].company, "Acme");
        assert_eq!(read_back[0].position, "Engineer");
        assert_eq!(read_back[1].status, "Interviewing");
        assert_eq!(read_back[0].url.as_deref(), Some("https://example.com/job/123"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_missing_file_returns_empty() {
        let path = Path::new("/nonexistent/path/does-not-exist.xlsx");
        let result = read_applications(path).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn dedupe_key_is_case_insensitive_and_trimmed() {
        let a = sample_app("  Acme Corp ", " Software Engineer", "Applied");
        let b = sample_app("acme corp", "software engineer", "Interviewing");
        assert_eq!(a.dedupe_key(), b.dedupe_key());
    }

    #[test]
    fn atomic_write_leaves_no_temp_file_on_success() {
        let dir = std::env::temp_dir().join(format!("job-tracker-test-atomic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("atomic.xlsx");

        write_applications(&path, &[sample_app("Acme", "Engineer", "Applied")]).unwrap();
        assert!(path.exists());
        assert!(!temp_path_for(&path).exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn second_write_creates_a_backup_of_the_first() {
        let dir = std::env::temp_dir().join(format!("job-tracker-test-backup-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Applications.xlsx");

        write_applications(&path, &[sample_app("Acme", "Engineer", "Applied")]).unwrap();
        let backups_dir = dir.join("backups");
        assert!(!backups_dir.exists(), "first write has nothing to back up yet");

        write_applications(&path, &[sample_app("Acme", "Engineer", "Interviewing")]).unwrap();
        assert!(backups_dir.exists());
        let backup_files: Vec<_> = std::fs::read_dir(&backups_dir).unwrap().collect();
        assert_eq!(backup_files.len(), 1, "second write should back up the first version");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn old_backups_beyond_the_limit_are_pruned() {
        let dir = std::env::temp_dir().join(format!("job-tracker-test-prune-{}", std::process::id()));
        let backups_dir = dir.join("backups");
        std::fs::create_dir_all(&backups_dir).unwrap();

        for i in 0..(MAX_BACKUPS + 5) {
            std::fs::write(
                backups_dir.join(format!("Applications_2026010{i:02}-000000.xlsx")),
                b"x",
            )
            .unwrap();
        }

        prune_old_backups(&backups_dir, "Applications", "xlsx").unwrap();
        let remaining = std::fs::read_dir(&backups_dir).unwrap().count();
        assert_eq!(remaining, MAX_BACKUPS);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn csv_export_matches_saved_rows() {
        let dir = std::env::temp_dir().join(format!("job-tracker-test-csv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let xlsx_path = dir.join("Applications.xlsx");

        let rows = vec![
            sample_app("Acme", "Engineer", "Applied"),
            sample_app("Globex", "Designer", "Offered"),
        ];
        let csv_path = export_csv(&xlsx_path, &rows).unwrap();
        assert_eq!(csv_path, dir.join("Applications.csv"));

        let contents = std::fs::read_to_string(&csv_path).unwrap();
        let mut lines = contents.lines();
        assert_eq!(lines.next().unwrap(), HEADERS.join(","));
        assert!(lines.next().unwrap().contains("Acme"));
        assert!(lines.next().unwrap().contains("Globex"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn excel_serial_dates_convert_to_iso() {
        // Known anchors: 1 = 1899-12-31 under Excel's off-by-one epoch,
        // 45000 = 2023-03-15, 46266 = 2026-09-01.
        assert_eq!(excel_serial_to_iso(46266.0).unwrap(), "2026-09-01");
        assert_eq!(excel_serial_to_iso(45000.0).unwrap(), "2023-03-15");
        // A date cell carrying a time of day keeps only the day.
        assert_eq!(excel_serial_to_iso(46266.75).unwrap(), "2026-09-01");
    }

    /// A user can format the Date Applied column as a Date in Excel - or type
    /// a date into a fresh cell, which Excel converts on their behalf. The
    /// cell then holds a day count, not text, and everything downstream
    /// (the Date column, the calendar, the CSV) needs `YYYY-MM-DD`.
    #[test]
    fn a_date_typed_cell_reads_back_as_an_iso_date() {
        let dir = std::env::temp_dir().join(format!(
            "job-tracker-datecell-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("DateTyped.xlsx");
        let _ = std::fs::remove_file(&path);

        // Build a workbook by hand with a real date-formatted cell, the way
        // Excel would leave it - not the string this app writes.
        {
            let mut workbook = Workbook::new();
            let sheet = workbook.add_worksheet();
            sheet.set_name(SHEET_NAME).unwrap();
            for (col, header) in HEADERS.iter().enumerate() {
                sheet.write_string(0, col as u16, *header).unwrap();
            }
            let date_format = Format::new().set_num_format("yyyy-mm-dd");
            let applied = rust_xlsxwriter::ExcelDateTime::from_ymd(2026, 9, 1).unwrap();
            let updated = rust_xlsxwriter::ExcelDateTime::from_ymd(2026, 9, 15).unwrap();
            sheet.write_datetime_with_format(1, 0, &applied, &date_format).unwrap();
            sheet.write_string(1, 1, "Amazon").unwrap();
            sheet.write_string(1, 2, "SDE Intern").unwrap();
            sheet.write_string(1, 7, "Applied").unwrap();
            sheet.write_datetime_with_format(1, 8, &updated, &date_format).unwrap();
            workbook.save(&path).unwrap();
        }

        let rows = read_applications(&path).expect("a hand-formatted workbook should still read");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].date_applied, "2026-09-01",
            "a date-typed cell must not come back as a serial number"
        );
        assert_eq!(rows[0].last_updated, "2026-09-15");
        assert_eq!(rows[0].company, "Amazon");

        let _ = std::fs::remove_file(&path);
    }
}
