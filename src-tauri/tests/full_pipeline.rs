//! Integration tests that exercise the real command functions (the same
//! functions the frontend calls via `invoke`) against a real, temporary
//! `.xlsx` file and a real `tauri::AppHandle` backed by the store and
//! clipboard-manager plugins. Unlike the unit tests in `src/excel.rs`,
//! these go through the actual Tauri command layer end to end.

use base64::Engine;
use job_tracker_lib::commands;
use job_tracker_lib::models::{ExtractedFields, ExtractionResult, JobApplication, SaveResult};

/// The settings store is a real file shared by every mock app in this
/// process, and cargo runs these tests in parallel. Anything that reads or
/// writes the status list takes this first, so one test cannot observe
/// another's list mid-edit. Poisoning is ignored - a panic in one test
/// should surface as that test's own failure, not as a lock error in
/// every other one.
static STATUS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock_statuses() -> std::sync::MutexGuard<'static, ()> {
    STATUS_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

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
    let _guard = lock_statuses();
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

    // The status list is persisted, and the store file is shared by every
    // mock app in this process, so a test that edits it would otherwise leak
    // into this one. Set what this test expects, the same way it sets the
    // workbook path.
    commands::set_status_defs(handle.clone(), job_tracker_lib::models::default_status_defs())
        .expect("defaults should be accepted");
    assert_eq!(
        commands::get_statuses(handle.clone()),
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

/// Deleting is the one write the app can't undo from inside itself, and it
/// addresses rows by index while the workbook is a file the user can edit in
/// Excel at the same time. Both halves of that are exercised here.
#[test]
fn deleting_a_row_removes_only_that_row() {
    let app = build_test_app();
    let handle = app.handle().clone();

    let dir = temp_dir_for("delete");
    let xlsx_path = dir.join("Delete.xlsx");
    let _ = std::fs::remove_file(&xlsx_path);
    commands::set_excel_path(handle.clone(), xlsx_path.to_string_lossy().to_string()).unwrap();

    let mut second = amazon_application();
    second.company = "Stripe".into();
    second.position = "Backend Engineer".into();
    let mut third = amazon_application();
    third.company = "Figma".into();
    third.position = "Product Engineer".into();

    for row in [amazon_application(), second, third] {
        commands::save_application(handle.clone(), row, false).unwrap();
    }
    assert_eq!(commands::list_applications(handle.clone()).unwrap().len(), 3);

    // Delete the middle row: the ones on either side must survive, and the
    // survivors must close up so index 1 is now what index 2 was.
    commands::delete_application_at_index(
        handle.clone(),
        1,
        "Stripe".into(),
        "Backend Engineer".into(),
    )
    .expect("deleting an existing row should succeed");

    let rows = commands::list_applications(handle.clone()).unwrap();
    assert_eq!(rows.len(), 2, "exactly one row should be gone");
    assert_eq!(rows[0].company, "Amazon");
    assert_eq!(rows[1].company, "Figma", "later rows shift down");
    assert!(
        !rows.iter().any(|r| r.company == "Stripe"),
        "the deleted row must not survive the round-trip through the workbook"
    );
}

#[test]
fn deleting_refuses_when_the_row_is_not_the_one_the_user_saw() {
    let app = build_test_app();
    let handle = app.handle().clone();

    let dir = temp_dir_for("delete-guard");
    let xlsx_path = dir.join("DeleteGuard.xlsx");
    let _ = std::fs::remove_file(&xlsx_path);
    commands::set_excel_path(handle.clone(), xlsx_path.to_string_lossy().to_string()).unwrap();

    commands::save_application(handle.clone(), amazon_application(), false).unwrap();

    // The index exists, but the row at it is something else entirely - what
    // you get if the workbook was re-sorted or edited in Excel meanwhile.
    let err = commands::delete_application_at_index(
        handle.clone(),
        0,
        "Some Other Company".into(),
        "Some Other Role".into(),
    )
    .expect_err("a mismatched row must be refused");
    assert!(
        err.contains("not the one you asked to delete"),
        "unexpected error: {err}"
    );
    assert_eq!(
        commands::list_applications(handle.clone()).unwrap().len(),
        1,
        "a refused delete must leave the workbook untouched"
    );

    // An index past the end is refused too, rather than panicking.
    let err = commands::delete_application_at_index(
        handle.clone(),
        99,
        "Amazon".into(),
        "Whatever".into(),
    )
    .expect_err("an out-of-range index must be refused");
    assert!(err.contains("no longer exists"), "unexpected error: {err}");
}

/// The status list is user-editable, and rows already carrying a status that
/// was removed must survive it - the workbook is the user's record.
#[test]
fn editing_the_status_list_leaves_existing_rows_alone() {
    use job_tracker_lib::models::{StatusDef, StatusKind};

    let _guard = lock_statuses();
    let app = build_test_app();
    let handle = app.handle().clone();

    let dir = temp_dir_for("statuses");
    let xlsx_path = dir.join("Statuses.xlsx");
    let _ = std::fs::remove_file(&xlsx_path);
    commands::set_excel_path(handle.clone(), xlsx_path.to_string_lossy().to_string()).unwrap();

    let mut row = amazon_application();
    row.status = "Ghosted".into();
    commands::save_application(handle.clone(), row, false).unwrap();

    // Replace the list with something narrower that drops "Ghosted".
    let custom = vec![
        StatusDef::new("Applied", StatusKind::Waiting),
        StatusDef::new("Phone screen", StatusKind::Replied),
        StatusDef::new("Offered", StatusKind::Replied),
    ];
    let saved = commands::set_status_defs(handle.clone(), custom.clone())
        .expect("a valid list should be accepted");
    assert_eq!(saved, custom);
    assert_eq!(
        commands::get_statuses(handle.clone()),
        vec!["Applied", "Phone screen", "Offered"]
    );

    let rows = commands::list_applications(handle.clone()).unwrap();
    assert_eq!(
        rows[0].status, "Ghosted",
        "a row keeps a status the list no longer offers"
    );

    // A save after the change must still work, and must not rewrite that row.
    let mut second = amazon_application();
    second.company = "Stripe".into();
    second.status = "Phone screen".into();
    commands::save_application(handle.clone(), second, false).unwrap();
    let rows = commands::list_applications(handle.clone()).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].status, "Ghosted");
    assert_eq!(rows[1].status, "Phone screen");

    // The store is shared across mock apps in this process; put the defaults
    // back so this test cannot colour another one.
    commands::set_status_defs(handle, job_tracker_lib::models::default_status_defs()).unwrap();
}

#[test]
fn an_unusable_status_list_is_refused() {
    use job_tracker_lib::models::{StatusDef, StatusKind};

    let _guard = lock_statuses();
    let app = build_test_app();
    let handle = app.handle().clone();

    commands::set_status_defs(handle.clone(), job_tracker_lib::models::default_status_defs())
        .expect("defaults should be accepted");

    assert!(commands::set_status_defs(handle.clone(), vec![]).is_err());
    assert!(commands::set_status_defs(
        handle.clone(),
        vec![StatusDef::new("   ", StatusKind::Waiting)]
    )
    .is_err());

    // A refused edit must leave the previous list in place.
    assert_eq!(
        commands::get_statuses(handle.clone()),
        vec!["Applied", "Interviewing", "Offered", "Rejected", "Ghosted", "Withdrawn"]
    );
}

/// Undo after a delete puts the row back where it was.
#[test]
fn a_deleted_row_can_be_put_back() {
    let app = build_test_app();
    let handle = app.handle().clone();

    let dir = temp_dir_for("undo");
    let xlsx_path = dir.join("Undo.xlsx");
    let _ = std::fs::remove_file(&xlsx_path);
    commands::set_excel_path(handle.clone(), xlsx_path.to_string_lossy().to_string()).unwrap();

    let mut second = amazon_application();
    second.company = "Stripe".into();
    let mut third = amazon_application();
    third.company = "Figma".into();
    for row in [amazon_application(), second, third] {
        commands::save_application(handle.clone(), row, false).unwrap();
    }

    let removed = commands::list_applications(handle.clone()).unwrap()[1].clone();
    commands::delete_application_at_index(handle.clone(), 1, removed.company.clone(), removed.position.clone())
        .unwrap();
    assert_eq!(commands::list_applications(handle.clone()).unwrap().len(), 2);

    commands::insert_application_at_index(handle.clone(), 1, removed.clone()).unwrap();
    let rows = commands::list_applications(handle.clone()).unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1].company, "Stripe", "it goes back where it was");
    assert_eq!(rows[0].company, "Amazon");
    assert_eq!(rows[2].company, "Figma");

    // An index past the end lands at the end rather than failing - by the
    // time Undo is clicked the workbook may have fewer rows than before.
    commands::insert_application_at_index(handle.clone(), 99, removed).unwrap();
    let rows = commands::list_applications(handle.clone()).unwrap();
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[3].company, "Stripe");
}

/// Importing merges another workbook without touching it or duplicating.
#[test]
fn importing_merges_and_skips_duplicates() {
    let app = build_test_app();
    let handle = app.handle().clone();

    let dir = temp_dir_for("import");
    let target = dir.join("Target.xlsx");
    let source = dir.join("Source.xlsx");
    let _ = std::fs::remove_file(&target);
    let _ = std::fs::remove_file(&source);

    // Build the source workbook through the same commands, then point the
    // app back at its own workbook.
    commands::set_excel_path(handle.clone(), source.to_string_lossy().to_string()).unwrap();
    let mut shared = amazon_application();
    shared.company = "Amazon".into();
    let mut only_in_source = amazon_application();
    only_in_source.company = "Datadog".into();
    only_in_source.position = "SRE".into();
    for row in [shared.clone(), only_in_source] {
        commands::save_application(handle.clone(), row, false).unwrap();
    }

    commands::set_excel_path(handle.clone(), target.to_string_lossy().to_string()).unwrap();
    commands::save_application(handle.clone(), shared, false).unwrap();

    let summary = commands::import_applications(handle.clone(), source.to_string_lossy().to_string())
        .expect("import should succeed");
    assert_eq!(summary.imported, 1, "only the row we did not already have");
    assert_eq!(summary.skipped_duplicates, 1);

    let rows = commands::list_applications(handle.clone()).unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|r| r.company == "Datadog"));

    // The source file must be left exactly as it was.
    commands::set_excel_path(handle.clone(), source.to_string_lossy().to_string()).unwrap();
    assert_eq!(
        commands::list_applications(handle.clone()).unwrap().len(),
        2,
        "importing must not modify the file it read from"
    );
}

#[test]
fn importing_the_workbook_you_are_already_using_is_refused() {
    let app = build_test_app();
    let handle = app.handle().clone();

    let dir = temp_dir_for("import-self");
    let path = dir.join("Self.xlsx");
    let _ = std::fs::remove_file(&path);
    commands::set_excel_path(handle.clone(), path.to_string_lossy().to_string()).unwrap();
    commands::save_application(handle.clone(), amazon_application(), false).unwrap();

    let err = commands::import_applications(handle.clone(), path.to_string_lossy().to_string())
        .expect_err("importing the active workbook into itself must be refused");
    assert!(err.contains("already tracking"), "unexpected error: {err}");
    assert_eq!(commands::list_applications(handle.clone()).unwrap().len(), 1);
}

/// The archive `save_screenshot` writes has to be findable again, and the
/// lookup has to build the same name the writer did.
#[test]
fn an_archived_screenshot_is_found_again() {
    use base64::Engine;

    let app = build_test_app();
    let handle = app.handle().clone();

    let dir = temp_dir_for("shot-lookup");
    let xlsx_path = dir.join("Shots.xlsx");
    let _ = std::fs::remove_dir_all(dir.join("JobApplications_screenshots"));
    commands::set_excel_path(handle.clone(), xlsx_path.to_string_lossy().to_string()).unwrap();

    let row = amazon_application();
    // Nothing archived yet.
    assert_eq!(
        commands::screenshot_for_application(
            handle.clone(),
            row.company.clone(),
            row.position.clone(),
            row.date_applied.clone()
        )
        .unwrap(),
        None,
        "a row with no capture behind it must report no screenshot"
    );

    // A 1x1 PNG is enough - only the naming is under test.
    let png = base64::engine::general_purpose::STANDARD.encode([
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
    ]);
    let written = commands::save_screenshot(
        handle.clone(),
        row.company.clone(),
        row.position.clone(),
        row.date_applied.clone(),
        png,
        "image/png".into(),
    )
    .expect("archiving should succeed");

    let found = commands::screenshot_for_application(
        handle.clone(),
        row.company.clone(),
        row.position.clone(),
        row.date_applied.clone(),
    )
    .unwrap()
    .expect("the archived screenshot must be findable");
    assert_eq!(found, written, "the lookup must resolve to the file just written");

    // A position with characters the filename sanitiser rewrites still
    // round-trips, since both sides go through the same helper.
    assert!(
        row.position.contains('/'),
        "this fixture is meant to contain a character that gets sanitised"
    );

    // A different row must not match this file.
    assert_eq!(
        commands::screenshot_for_application(
            handle,
            "Someone Else".into(),
            row.position.clone(),
            row.date_applied
        )
        .unwrap(),
        None
    );
}

/// The model is a setting, so a provider retiring one doesn't need a release.
#[test]
fn a_provider_model_can_be_overridden_and_cleared() {
    let app = build_test_app();
    let handle = app.handle().clone();

    // Starts at the shipped default.
    let shipped = job_tracker_lib::models::provider_or_default("openai").default_model;
    assert!(!shipped.is_empty());
    assert_eq!(commands::get_model(handle.clone(), "openai".into()), shipped);

    let set = commands::set_model(handle.clone(), "openai".into(), "gpt-5-vision".into()).unwrap();
    assert_eq!(set, "gpt-5-vision");
    assert_eq!(commands::get_model(handle.clone(), "openai".into()), "gpt-5-vision");

    // One provider's override must not touch another's.
    assert_eq!(
        commands::get_model(handle.clone(), "gemini".into()),
        job_tracker_lib::models::provider_or_default("gemini").default_model
    );

    // Blanking it goes back to the default rather than sending an empty model.
    let cleared = commands::set_model(handle.clone(), "openai".into(), "   ".into()).unwrap();
    assert_eq!(cleared, shipped);
    assert_eq!(commands::get_model(handle, "openai".into()), shipped);
}
