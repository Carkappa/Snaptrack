//! Exercises the operating system's own OCR engine against a real image.
//!
//! Calls the engine directly rather than through a command, so it needs no
//! Tauri app and runs on a plain Windows machine. Skipped where there is no
//! system engine, so it passes on the Linux CI runner without pretending to
//! have tested anything there - but never on macOS or Windows, where an
//! engine is part of the OS and its absence is a real failure rather than
//! a machine without one. Skipping quietly everywhere is how this test
//! managed to run nowhere at all before the macos job existed.

/// Whether to run, refusing to skip where the OS guarantees an engine.
///
/// Windows has had one since Windows 10 and macOS since 10.15, so on those
/// two a missing engine means the wrapper is broken, not that the machine
/// is unusual. Linux has none and skips for real.
fn engine_or_skip() -> bool {
    if job_tracker_lib::system_ocr::available() {
        return true;
    }
    assert!(
        !cfg!(any(target_os = "macos", target_os = "windows")),
        "this OS ships an OCR engine, so finding none is a failure of the wrapper rather than a reason to skip"
    );
    eprintln!("no system OCR engine on this machine - skipping");
    false
}

#[tokio::test]
async fn the_system_engine_reads_a_real_screenshot() {
    if !engine_or_skip() {
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

    // The card reads: company, then the job title, then the metadata line.
    // Asserting on that order rather than on `top` being sorted, because
    // the sort happens either way - it is the only thing here that catches
    // a `top` measured from the wrong edge. macOS matters for this: Vision
    // reports its boxes with the origin at the bottom left, and an
    // un-flipped read would hand back a perfectly sorted card upside down,
    // leaving the field heuristics to call the last line the job title.
    let first_containing = |needle: &str| {
        blocks
            .iter()
            .position(|b| b.text.to_lowercase().contains(needle))
    };
    let company = first_containing("atricure").expect("the company line");
    let title = first_containing("co-op").expect("the title line");
    let footer = first_containing("promoted").expect("the metadata line");
    assert!(
        company < title && title < footer,
        "the card must come back the way it is laid out, got: {:?}",
        blocks.iter().map(|b| &b.text).collect::<Vec<_>>()
    );
    assert!(
        blocks.iter().any(|b| b.height > 0.0),
        "at least one block must carry a real text height"
    );
}

#[tokio::test]
async fn junk_input_is_reported_rather_than_panicking() {
    if !engine_or_skip() {
        return;
    }
    assert!(job_tracker_lib::system_ocr::run(b"not an image").await.is_err());
    assert!(job_tracker_lib::system_ocr::run(&[]).await.is_err());
}
