//! Compiling a `.tex` with whatever TeX the machine already has.
//!
//! Keeping someone's LaTeX style only half worked without this. The `.tex`
//! came out in their document and the PDF - the file that gets attached to
//! the application row, and the one actually sent - came out of the
//! built-in writer in a style they never chose. Someone who keeps their
//! resume in LaTeX usually has a TeX distribution too, so where there is
//! one, the PDF is theirs as well.
//!
//! Nothing here is required. No engine, a failed run, a preamble that
//! wants a package that is not installed - all of it falls back to the
//! built-in PDF with a reason, because a resume in the wrong style beats
//! no resume at all.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// How long a run may take before it is killed.
///
/// A first run that has to pull fonts is slow, and a broken document can
/// wait forever for input that is never coming - `-interaction=nonstopmode`
/// covers most of that, but not a package that prompts on its own.
const TIMEOUT: Duration = Duration::from_secs(90);

/// Engines that can produce a PDF directly, best-suited first.
///
/// `fontspec` and its relatives need a Unicode engine; pdflatex fails on
/// them outright. Plenty of resume templates use it, so the preamble picks
/// the order rather than a fixed preference.
const UNICODE_ENGINES: [&str; 3] = ["xelatex", "lualatex", "tectonic"];
const ANY_ENGINE: [&str; 4] = ["pdflatex", "xelatex", "lualatex", "tectonic"];

/// Whether a preamble needs a Unicode-capable engine.
pub fn needs_unicode_engine(preamble: &str) -> bool {
    ["fontspec", "unicode-math", "polyglossia", "xeCJK", "luatexja"]
        .iter()
        .any(|package| preamble.contains(package))
}

/// The engines worth trying for this document, in order.
pub fn candidates(preamble: &str) -> Vec<&'static str> {
    if needs_unicode_engine(preamble) {
        UNICODE_ENGINES.to_vec()
    } else {
        ANY_ENGINE.to_vec()
    }
}

/// Whether a named engine is on PATH.
fn installed(engine: &str) -> bool {
    // Tectonic answers --version like the rest; every candidate here does.
    silent(Command::new(engine).arg("--version"))
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// The first candidate engine this machine actually has.
pub fn find_engine(preamble: &str) -> Option<&'static str> {
    candidates(preamble).into_iter().find(|e| installed(e))
}

/// Runs a command without letting a console window flash up.
///
/// The app is built with `windows_subsystem = "windows"`, which stops it
/// opening a console of its own - but a *child* process still gets one
/// unless it is asked not to, and a black rectangle appearing over the app
/// mid-save reads as a crash.
fn silent(command: &mut Command) -> std::io::Result<std::process::Output> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.output()
}

/// Spawns a command, and kills it if it outstays `TIMEOUT`.
fn run_bounded(command: &mut Command) -> Result<std::process::Output, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|e| format!("could not start it ({e})"))?;

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if started.elapsed() > TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("it took too long and was stopped".to_string());
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("it could not be waited on ({e})")),
        }
    }

    child
        .wait_with_output()
        .map_err(|e| format!("its output could not be read ({e})"))
}

/// The first real error in a TeX log.
///
/// TeX errors start with `!` at the beginning of a line, and the line
/// after usually carries the detail. Everything before the first one is
/// banner and package chatter that explains nothing.
pub fn first_error(log: &str) -> String {
    let mut lines = log.lines().skip_while(|l| !l.starts_with('!'));
    let Some(first) = lines.next() else {
        return "it reported no error but produced no PDF".to_string();
    };
    let detail = lines
        .find(|l| !l.trim().is_empty() && !l.starts_with("l."))
        .unwrap_or("");
    let message = format!("{} {}", first.trim_start_matches('!').trim(), detail.trim());
    let message = message.trim();
    // Long enough to name the missing package, short enough for one line
    // of a hint under a button.
    message.chars().take(160).collect::<String>().trim().to_string()
}

/// Whether the engine asked to be run again - a document with references
/// or a page count gets them right only on the second pass.
fn wants_rerun(log: &str) -> bool {
    log.contains("Rerun to get") || log.contains("Rerun LaTeX")
}

/// Compiles `tex` and returns the PDF bytes.
///
/// `resources` is the folder the author's own `.tex` lives in, put on
/// TEXINPUTS so a template that `\input`s a file or includes a photograph
/// still finds it. Compilation happens in a scratch directory so the
/// `.aux`, `.log` and `.out` droppings never land in the user's Resumes
/// folder next to the files they actually want.
pub fn compile(tex: &str, engine: &str, resources: Option<&Path>) -> Result<Vec<u8>, String> {
    let dir = scratch_dir()?;
    let result = compile_in(&dir, tex, engine, resources);
    // Best-effort: a locked file on Windows should not turn a successful
    // compile into a failure.
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn scratch_dir() -> Result<PathBuf, String> {
    let unique = format!(
        "job-tracker-tex-{}-{}",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    );
    let dir = std::env::temp_dir().join(unique);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("a working folder could not be made ({e})"))?;
    Ok(dir)
}

fn compile_in(
    dir: &Path,
    tex: &str,
    engine: &str,
    resources: Option<&Path>,
) -> Result<Vec<u8>, String> {
    let source = dir.join("resume.tex");
    std::fs::write(&source, tex).map_err(|e| format!("it could not be written out ({e})"))?;

    for pass in 0..2 {
        let mut command = Command::new(engine);
        command.current_dir(dir);
        if engine == "tectonic" {
            command.arg("--outdir").arg(dir).arg(&source);
        } else {
            command
                .arg("-interaction=nonstopmode")
                .arg("-halt-on-error")
                .arg("-output-directory")
                .arg(dir)
                .arg(&source);
        }
        if let Some(resources) = resources {
            // A trailing separator means "and the usual places too"; without
            // it this replaces the distribution's own search path and
            // nothing at all is found.
            let separator = if cfg!(windows) { ";" } else { ":" };
            command.env(
                "TEXINPUTS",
                format!("{}{}{}", resources.display(), separator, separator),
            );
        }

        let output = run_bounded(&mut command)?;
        let log = std::fs::read_to_string(dir.join("resume.log")).unwrap_or_default();
        let combined = format!("{}{}", log, String::from_utf8_lossy(&output.stdout));

        let pdf = dir.join("resume.pdf");
        if !output.status.success() && !pdf.exists() {
            return Err(first_error(&combined));
        }
        if pass == 0 && wants_rerun(&combined) {
            continue;
        }
        return std::fs::read(&pdf)
            .map_err(|e| format!("it compiled but the PDF could not be read ({e})"));
    }
    unreachable!("the loop returns or continues exactly once")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fontspec_preamble_rules_out_pdflatex() {
        let preamble = r"\documentclass{article}\usepackage{fontspec}";
        assert!(needs_unicode_engine(preamble));
        assert!(!candidates(preamble).contains(&"pdflatex"));
        assert_eq!(candidates(preamble)[0], "xelatex");
    }

    #[test]
    fn a_plain_preamble_can_use_any_engine() {
        let preamble = r"\documentclass{article}\usepackage{geometry}";
        assert!(!needs_unicode_engine(preamble));
        assert_eq!(candidates(preamble)[0], "pdflatex");
        assert_eq!(candidates(preamble).len(), 4);
    }

    #[test]
    fn every_candidate_is_an_engine_that_writes_a_pdf_directly() {
        // latex(1) writes a DVI, which is no use here. Nothing in either
        // list may be one.
        for engine in ANY_ENGINE.iter().chain(UNICODE_ENGINES.iter()) {
            assert!(
                ["pdflatex", "xelatex", "lualatex", "tectonic"].contains(engine),
                "{engine} does not produce a PDF directly"
            );
        }
    }

    #[test]
    fn the_first_real_error_is_pulled_out_of_the_noise() {
        // Built from a list rather than one string with line continuations:
        // check-syntax.sh reads a trailing backslash as the broken-escape
        // bug it exists to catch, and it is right to.
        let log = [
            "This is XeTeX, Version 3.14",
            "(./resume.tex",
            "LaTeX2e <2023-11-01>",
            "! LaTeX Error: File `moderncv.cls' not found.",
            "",
            "Type X to quit.",
            "l.1 \\documentclass{moderncv}",
        ]
        .join("\n");
        let error = first_error(&log);
        assert!(error.contains("moderncv.cls"), "got: {error}");
        assert!(!error.contains("XeTeX"), "the banner is not the error: {error}");
    }

    #[test]
    fn a_log_with_no_error_still_says_something_useful() {
        let error = first_error("This is pdfTeX\n(./resume.tex)\n");
        assert!(!error.is_empty());
        assert!(error.contains("no error"), "got: {error}");
    }

    #[test]
    fn an_error_message_stays_short_enough_to_show() {
        let log = format!("! LaTeX Error: {}\n{}", "x".repeat(400), "y".repeat(400));
        assert!(first_error(&log).chars().count() <= 160);
    }

    #[test]
    fn a_rerun_request_is_recognised_but_ordinary_logs_are_not() {
        assert!(wants_rerun("LaTeX Warning: Label(s) may have changed. Rerun to get them right."));
        assert!(!wants_rerun("Output written on resume.pdf (1 page)."));
    }
}
