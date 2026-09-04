//! Compiling a real `.tex` with a real TeX engine.
//!
//! Skips where no engine is installed, the same way `tests/system_ocr.rs`
//! skips where there is no OS OCR engine. That means it does nothing on a
//! bare development machine, so the `rust` job in `test.yml` installs a
//! minimal TeX Live to make sure it runs somewhere - without that, the
//! whole compile path would be covered only by unit tests of its string
//! handling, which is the part least likely to be wrong.

use job_tracker_lib::latex_build;

/// Small, but not a toy: a document class, a package, a custom macro and
/// an environment, which is the shape the real templates have.
const DOCUMENT: &str = r#"\documentclass[11pt]{article}
\usepackage[margin=1in]{geometry}
\newcommand{\heading}[1]{\textbf{\large #1}\par\vspace{2pt}\hrule\vspace{4pt}}
\pagestyle{empty}
\begin{document}
\begin{center}{\LARGE Jun Du}\\ jun@example.com\end{center}
\heading{Experience}
\begin{itemize}
  \item Built a job application tracker.
\end{itemize}
\end{document}
"#;

fn engine() -> Option<&'static str> {
    latex_build::find_engine("")
}

#[test]
fn a_real_document_compiles_to_a_real_pdf() {
    let Some(engine) = engine() else {
        eprintln!("skipped: no LaTeX engine on this machine");
        return;
    };

    let pdf = latex_build::compile(DOCUMENT, engine, None)
        .unwrap_or_else(|e| panic!("{engine} could not build the document: {e}"));

    // A PDF, not an empty file or a log that happened to be readable.
    assert!(pdf.starts_with(b"%PDF-"), "not a PDF: {:?}", &pdf[..8.min(pdf.len())]);
    assert!(
        pdf.len() > 1000,
        "a one-page resume should not be {} bytes",
        pdf.len()
    );
    assert!(
        pdf.windows(5).any(|w| w == b"%%EOF"),
        "the PDF is truncated - no end marker"
    );
}

#[test]
fn a_broken_document_fails_with_the_reason_rather_than_hanging() {
    let Some(engine) = engine() else {
        eprintln!("skipped: no LaTeX engine on this machine");
        return;
    };

    // A class that does not exist. Without -interaction=nonstopmode this
    // is exactly the document that sits waiting for input forever.
    let broken = r"\documentclass{no-such-class-anywhere}\begin{document}x\end{document}";
    let error = latex_build::compile(broken, engine, None)
        .expect_err("a missing document class must not look like success");

    assert!(!error.is_empty(), "the failure has to say something");
    assert!(
        error.to_lowercase().contains("no-such-class-anywhere")
            || error.to_lowercase().contains("file")
            || error.to_lowercase().contains("class"),
        "the reason should name the problem, got: {error}"
    );
}

#[test]
fn compiling_leaves_nothing_behind_in_the_temp_directory() {
    let Some(engine) = engine() else {
        eprintln!("skipped: no LaTeX engine on this machine");
        return;
    };

    let before = scratch_folders();
    let _ = latex_build::compile(DOCUMENT, engine, None);
    let after = scratch_folders();

    // The .aux, .log and .out files a TeX run drops must not accumulate,
    // and must never land beside the resumes the user actually wants.
    assert_eq!(
        before, after,
        "a scratch folder was left behind after compiling"
    );
}

fn scratch_folders() -> usize {
    std::fs::read_dir(std::env::temp_dir())
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("job-tracker-tex-")
                })
                .count()
        })
        .unwrap_or(0)
}
