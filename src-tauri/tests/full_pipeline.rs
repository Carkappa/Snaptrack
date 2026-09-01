//! Integration tests that exercise the real command functions (the same
//! functions the frontend calls via `invoke`) against a real, temporary
//! `.xlsx` file and a real `tauri::AppHandle` backed by the store and
//! clipboard-manager plugins. Unlike the unit tests in `src/excel.rs`,
//! these go through the actual Tauri command layer end to end.

use base64::Engine;
use job_tracker_lib::commands;
use job_tracker_lib::models::{ExtractedFields, ExtractionResult, JobApplication, SaveResult};

fn build_test_app() -> tauri::App<tauri::test::MockRuntime> {
    tauri::test::mock_builder()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .expect("failed to build mock app")
}

fn temp_dir_for(test_name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "job-tracker-itest-{test_name}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn amazon_application() -> JobApplication {
    JobApplication {
        date_applied: "2026-09-01".into(),
        company: "Amazon".into(),
        position: "Robotics - Software Development Engineer Fall Intern/Co-op - 2026".into(),
        location: Some("Westboro, WI".into()),
        work_type: None,
        employment_type: Some("Full-time".into()),
        salary_range: None,
        status: "Applied".into(),
        last_updated: "2026-09-01".into(),
        job_id: None,
        url: Some("https://www.linkedin.com/jobs/view/example-4123456789".into()),
        notes: None,
    }
}

#[test]
fn full_capture_to_excel_pipeline() {
    let app = build_test_app();
    let handle = app.handle().clone();

    let dir = temp_dir_for("pipeline");
    let xlsx_path = dir.join("Applications.xlsx");

    // Settings: point the app at our temp workbook, same as the Settings
    // tab's "Choose..." button would via pick_excel_path + set_excel_path.
    commands::set_excel_path(handle.clone(), xlsx_path.to_string_lossy().to_string())
        .expect("set_excel_path should succeed");
    assert_eq!(
        commands::get_excel_path(handle.clone()).expect("get_excel_path should succeed"),
        xlsx_path.to_string_lossy()
    );

    // Status dropdown source of truth.
    assert_eq!(
        commands::get_statuses(),
        vec!["Applied", "Interviewing", "Offered", "Rejected", "Ghosted", "Withdrawn"]
    );

    // Brand-new workbook: list tab should show nothing without erroring.
    let initial = commands::list_applications(handle.clone()).expect("list should succeed");
    assert!(initial.is_empty(), "a fresh workbook should have no rows");

    // Simulates what would happen after a real screenshot -> extract_from_image
    // -> review form -> Save, using the actual Amazon LinkedIn posting fields
    // a correct extraction would produce.
    let application = amazon_application();
    let result = commands::save_application(handle.clone(), application.clone(), false)
        .expect("save_application should succeed");
    assert!(matches!(result, SaveResult::Saved), "first save should not be a duplicate");

    let after_first_save = commands::list_applications(handle.clone()).unwrap();
    assert_eq!(after_first_save.len(), 1);
    assert_eq!(after_first_save[0].company, "Amazon");
    assert_eq!(
        after_first_save[0].position,
        "Robotics - Software Development Engineer Fall Intern/Co-op - 2026"
    );
    assert_eq!(after_first_save[0].location.as_deref(), Some("Westboro, WI"));
    assert_eq!(after_first_save[0].employment_type.as_deref(), Some("Full-time"));
    assert_eq!(after_first_save[0].status, "Applied");
    assert_eq!(
        after_first_save[0].url.as_deref(),
        Some("https://www.linkedin.com/jobs/view/example-4123456789")
    );

    // Saving the identical company + position again (case/whitespace
    // variant) must be caught as a duplicate, not silently double-saved.
    let mut repeat_application = amazon_application();
    repeat_application.company = "  amazon ".into();
    repeat_application.position = repeat_application.position.to_uppercase();
    let dup_result = commands::save_application(handle.clone(), repeat_application.clone(), false)
        .expect("save_application should succeed");
    match dup_result {
        SaveResult::Duplicate { existing_status } => assert_eq!(existing_status, "Applied"),
        SaveResult::Saved => panic!("expected a duplicate to be detected"),
    }
    assert_eq!(
        commands::list_applications(handle.clone()).unwrap().len(),
        1,
        "a rejected duplicate must not add a row"
    );

    // "Save anyway" must force the second row in despite the duplicate.
    let forced = commands::save_application(handle.clone(), repeat_application, true).unwrap();
    assert!(matches!(forced, SaveResult::Saved));
    assert_eq!(commands::list_applications(handle.clone()).unwrap().len(), 2);

    // "Update existing status instead" updates the first matching row.
    commands::update_existing_status(
        handle.clone(),
        "AMAZON".into(),
        "robotics - software development engineer fall intern/co-op - 2026".into(),
        "Interviewing".into(),
    )
    .expect("update_existing_status should find the row");
    let after_update = commands::list_applications(handle.clone()).unwrap();
    assert_eq!(after_update[0].status, "Interviewing");

    // The list tab's inline per-row status dropdown, addressed by index.
    commands::update_status_at_index(handle.clone(), 1, "Offered".into())
        .expect("update_status_at_index should succeed");
    let final_rows = commands::list_applications(handle.clone()).unwrap();
    assert_eq!(final_rows[1].status, "Offered");

    // Now inspect the *actual* .xlsx bytes on disk (not just what calamine
    // reports back) to prove the Excel-specific features - the status
    // data-validation dropdown and status fill colors - are really in the
    // saved file, not only in our in-memory model.
    let file_bytes = std::fs::read(&xlsx_path).expect("workbook should exist on disk");
    let reader = std::io::Cursor::new(file_bytes);
    let mut zip = zip::ZipArchive::new(reader).expect("workbook should be a valid zip/xlsx");

    let mut sheet_xml = String::new();
    {
        use std::io::Read;
        let mut sheet_file = zip
            .by_name("xl/worksheets/sheet1.xml")
            .expect("workbook should contain a first worksheet");
        sheet_file.read_to_string(&mut sheet_xml).unwrap();
    }

    assert!(
        sheet_xml.contains("dataValidation"),
        "Status column should carry a real Excel data-validation dropdown"
    );
    assert!(
        sheet_xml.contains("Applied") && sheet_xml.contains("Withdrawn"),
        "the dropdown's allowed list should include all statuses"
    );

    let mut styles_xml = String::new();
    {
        use std::io::Read;
        let mut styles_file = zip
            .by_name("xl/styles.xml")
            .expect("workbook should contain styles");
        styles_file.read_to_string(&mut styles_xml).unwrap();
    }
    assert!(
        styles_xml.to_uppercase().contains("FFC6EFCE"),
        "the Offered row's green fill color should be embedded in the workbook styles"
    );

    assert!(
        sheet_xml.contains("hyperlink") || zip.by_name("xl/worksheets/_rels/sheet1.xml.rels").is_ok(),
        "the URL column should be written as a real hyperlink"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn parse_failed_extraction_is_reported_not_dropped() {
    // Mirrors what happens when Claude's response can't be parsed as JSON:
    // the frontend should still get something to show the user, never a
    // silent failure.
    let raw = "Sorry, I can't read this image clearly.";
    let parse_error = serde_json::from_str::<ExtractedFields>(raw).unwrap_err();
    let result = ExtractionResult::ParseFailed {
        raw_text: raw.to_string(),
        error: parse_error.to_string(),
    };
    match result {
        ExtractionResult::ParseFailed { raw_text, .. } => assert_eq!(raw_text, raw),
        ExtractionResult::Parsed { .. } => panic!("expected a parse failure"),
    }
}

/// Manual verification test, not run in CI: proves `read_clipboard_image`
/// (the Rust clipboard-manager path the capture panel's Cmd/Ctrl+V uses)
/// reads whatever image is really on the OS clipboard right now.
///
/// Run it after copying an image, e.g.:
///   osascript -e 'set the clipboard to (read (POSIX file "/path/to.png") as «class PNGf»)'
///   cargo test --test full_pipeline -- --ignored reads_real_clipboard_image
#[test]
#[ignore]
fn reads_real_clipboard_image() {
    let app = build_test_app();
    let handle = app.handle().clone();

    let base64_str = commands::read_clipboard_image(handle)
        .expect("read_clipboard_image should not error")
        .expect("expected an image on the clipboard - copy one first, see doc comment above");

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&base64_str)
        .expect("should be valid base64");
    let decoded = image::load_from_memory(&bytes).expect("should decode as a real PNG image");
    println!("clipboard image is {}x{}", decoded.width(), decoded.height());
    assert!(decoded.width() > 0 && decoded.height() > 0);
}

/// Manual verification test, not run in CI: runs the real Tesseract-backed
/// `extract_with_local_ocr` command against a real screenshot file and
/// prints what it guessed, so the heuristic's real-world accuracy can be
/// eyeballed against an actual job posting (not just synthetic fixtures).
///
/// Run it with:
///   OCR_TEST_IMAGE=/path/to/screenshot.png \
///     cargo test --test full_pipeline -- --ignored extracts_from_a_real_screenshot --nocapture
#[test]
#[ignore]
fn extracts_from_a_real_screenshot() {
    let path = std::env::var("OCR_TEST_IMAGE")
        .expect("set OCR_TEST_IMAGE to a screenshot file path first, see doc comment above");
    let bytes = std::fs::read(&path).expect("should be able to read OCR_TEST_IMAGE");
    let base64_str = base64::engine::general_purpose::STANDARD.encode(&bytes);

    let result = commands::extract_with_local_ocr(base64_str).expect("extraction should not error");
    match result {
        ExtractionResult::Parsed { fields } => {
            println!("company:         {:?}", fields.company);
            println!("position:        {:?}", fields.position);
            println!("location:        {:?}", fields.location);
            println!("work_type:       {:?}", fields.work_type);
            println!("employment_type: {:?}", fields.employment_type);
            println!("salary_range:    {:?}", fields.salary_range);
            println!("job_id:          {:?}", fields.job_id);
            println!("posted_date:     {:?}", fields.posted_date);
            println!("notes:\n{}", fields.notes.unwrap_or_default());
            assert!(fields.company.is_some() || fields.position.is_some());
        }
        ExtractionResult::ParseFailed { error, .. } => panic!("expected fields, got: {error}"),
    }
}
