//! Exercises the operating system's own OCR engine against a real image.
//!
//! Calls the engine directly rather than through a command, so it needs no
//! Tauri app and runs on a plain Windows machine. Skipped where there is no
//! system engine, so it passes on the Linux CI runner without pretending to
//! have tested anything there.

#[tokio::test]
async fn the_system_engine_reads_a_real_screenshot() {
    if !job_tracker_lib::system_ocr::available() {
        eprintln!("no system OCR engine on this machine - skipping");
        return;
    }

    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/card.png");
    let bytes = std::fs::read(path).expect("the fixture image should be present");

    let blocks = job_tracker_lib::system_ocr::run(&bytes)
        .await
        .expect("the system engine should read a plain screenshot");

    assert!(!blocks.is_empty(), "an image with text must produce blocks");

    let all = blocks
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    assert!(
        all.contains("atricure"),
        "expected the company name, got: {:?}",
        blocks.iter().map(|b| &b.text).collect::<Vec<_>>()
    );
    assert!(
        all.contains("co-op") || all.contains("fall"),
        "expected the job title, got: {all}"
    );

    // The whole point of this engine here: no debris from the logo, which
    // is what Tesseract produced on this same image.
    assert!(
        !all.contains("mes5"),
        "the logo must not become text: {all}"
    );

    // Blocks come back in reading order with usable heights, which is what
    // the field heuristics need.
    assert!(
        blocks.windows(2).all(|w| w[0].top <= w[1].top),
        "blocks must be ordered top to bottom"
    );
    assert!(
        blocks.iter().any(|b| b.height > 0.0),
        "at least one block must carry a real text height"
    );
}

#[tokio::test]
async fn junk_input_is_reported_rather_than_panicking() {
    if !job_tracker_lib::system_ocr::available() {
        return;
    }
    assert!(job_tracker_lib::system_ocr::run(b"not an image").await.is_err());
    assert!(job_tracker_lib::system_ocr::run(&[]).await.is_err());
}
