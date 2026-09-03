//! Round-trips a real PDF through the importer.
//!
//! Written with the app's own PDF writer, so this tests actual extraction
//! rather than a hand-made fixture that might not resemble a resume.

use job_tracker_lib::resume;
use job_tracker_lib::resume_render::{Entry, Resume, Section};

fn sample() -> Resume {
    Resume {
        name: "Jun Du".into(),
        contact: "jun@example.com".into(),
        summary: "Software engineer.".into(),
        sections: vec![Section {
            heading: "Experience".into(),
            entries: vec![Entry {
                title: "Software Engineer".into(),
                organisation: "Acme".into(),
                dates: "2024 - 2026".into(),
                location: "Remote".into(),
                bullets: vec!["Built a job tracker".into()],
            }],
            items: vec![],
        }],
    }
}

#[test]
fn a_real_pdf_can_be_read_back_in() {
    let dir = std::env::temp_dir().join(format!("jt-resume-import-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("resume.pdf");

    let pdf = job_tracker_lib::resume_render::to_pdf(&sample()).expect("should render");
    std::fs::write(&path, pdf).unwrap();

    let text = resume::import_text(&path).expect("a PDF we wrote should be readable");
    assert!(
        text.contains("Jun Du"),
        "the name should survive the round trip, got: {text:?}"
    );
    assert!(text.contains("Acme"), "and so should the employer: {text:?}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_that_is_not_a_pdf_is_refused_clearly() {
    let dir = std::env::temp_dir().join(format!("jt-resume-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("notreally.pdf");
    std::fs::write(&path, b"this is not a pdf").unwrap();

    let err = resume::import_text(&path).expect_err("junk must not be accepted");
    assert!(
        err.to_lowercase().contains("pdf"),
        "the message should say what went wrong: {err}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn plain_text_needs_no_parsing() {
    let dir = std::env::temp_dir().join(format!("jt-resume-txt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("resume.md");
    std::fs::write(&path, "# Jun Du\n\n\n\nEngineer").unwrap();

    let text = resume::import_text(&path).unwrap();
    assert!(text.contains("Jun Du"));
    assert!(!text.contains("\n\n\n"), "blank runs are collapsed: {text:?}");

    let _ = std::fs::remove_dir_all(&dir);
}
