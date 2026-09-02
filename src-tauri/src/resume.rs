//! Tailoring a stored resume to a particular posting.
//!
//! The master resume is everything you have ever done, kept in one file
//! next to the workbook so you own it the same way. Tailoring reads that
//! plus a job description and produces a shorter one aimed at the role.
//!
//! The model is told to cut and reorder, never to invent - the same rule
//! the extraction prompt follows, and for a stronger reason: a fabricated
//! line on a resume is a lie told in your name, and you may not notice it
//! before sending.

use std::path::{Path, PathBuf};

/// What the model is allowed to do with a resume.
pub const SYSTEM_PROMPT: &str = r#"You tailor an existing resume to a specific job posting.

Absolute rules, in order of importance:
1. NEVER invent anything. No employer, job title, date, degree, metric, tool or achievement may appear in your output unless it appears in the master resume. If the posting wants something the candidate does not have, leave it out - do not manufacture it.
2. Do not exaggerate. "Familiar with X" must not become "expert in X". Numbers must not grow.
3. You may cut, reorder, and re-word. Prefer cutting: a shorter resume aimed at this posting beats a long one that mentions everything.
4. Keep the candidate's own phrasing where it already works.
5. Preserve every date range you keep, exactly as written. Employment gaps are not yours to hide.

Aim for one page of content unless the master resume is clearly for a senior role that needs two.

Return only the tailored resume as plain Markdown - no preamble, no explanation, no commentary about what you changed."#;

/// Where the master resume lives: beside the workbook, so it moves with it
/// and is as easy to open, edit and back up as the spreadsheet is.
pub fn master_path(workbook: &Path) -> Option<PathBuf> {
    let parent = workbook.parent().filter(|p| !p.as_os_str().is_empty())?;
    Some(parent.join("Resume_master.md"))
}

/// Folder for the tailored copies.
pub fn output_dir(workbook: &Path) -> Option<PathBuf> {
    let parent = workbook.parent().filter(|p| !p.as_os_str().is_empty())?;
    Some(parent.join("Resumes"))
}

/// Filename for a tailored resume, from the role it was aimed at.
///
/// Named after the job rather than the date, because the reason to open one
/// six weeks later is "what did I send Amazon?" - and the same job applied
/// for twice should not silently overwrite the first attempt.
pub fn output_name(company: &str, position: &str) -> String {
    let clean = |s: &str| {
        let kept: String = s
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect();
        // Words joined by single hyphens. Punctuation becomes a space
        // first, so "Robotics - Software" does not end up with three.
        kept.split_whitespace().collect::<Vec<_>>().join("-")
    };

    let company = clean(company);
    let position = clean(position);
    let stem = match (company.is_empty(), position.is_empty()) {
        (true, true) => "Resume".to_string(),
        (true, false) => position,
        (false, true) => company,
        (false, false) => format!("{company}-{position}"),
    };
    // Long titles are common and a 300-character filename is its own
    // problem on Windows.
    let trimmed: String = stem.chars().take(80).collect();
    format!("{}.md", trimmed.trim_matches('-'))
}

/// The prompt describing one posting, from whatever is known about it.
pub fn job_brief(
    company: &str,
    position: &str,
    location: &str,
    notes: &str,
    pasted: &str,
) -> String {
    let mut brief = String::new();
    if !company.trim().is_empty() {
        brief.push_str(&format!("Company: {}\n", company.trim()));
    }
    if !position.trim().is_empty() {
        brief.push_str(&format!("Role: {}\n", position.trim()));
    }
    if !location.trim().is_empty() {
        brief.push_str(&format!("Location: {}\n", location.trim()));
    }
    // The pasted description is the useful part when it exists; the notes
    // are usually raw OCR text and only worth including without one.
    if !pasted.trim().is_empty() {
        brief.push_str("\nPosting:\n");
        brief.push_str(pasted.trim());
    } else if !notes.trim().is_empty() {
        brief.push_str("\nWhat was captured from the posting:\n");
        brief.push_str(notes.trim());
    }
    brief
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filename_says_which_job_it_was_for() {
        assert_eq!(
            output_name("Amazon", "Robotics - Software Development Engineer"),
            "Amazon-Robotics-Software-Development-Engineer.md"
        );
        assert_eq!(output_name("AtriCure, Inc.", "IT Co-op"), "AtriCure-Inc-IT-Co-op.md");
    }

    #[test]
    fn characters_a_filesystem_refuses_are_removed() {
        let name = output_name("A/B\\C:D*E?F", "Engineer <Senior>");
        assert!(!name.contains(['/', '\\', ':', '*', '?', '<', '>']));
        assert!(name.ends_with(".md"));
    }

    #[test]
    fn a_very_long_title_does_not_become_a_very_long_filename() {
        let name = output_name("Company", &"Very Long Title ".repeat(20));
        assert!(name.chars().count() <= 84, "got {} chars", name.chars().count());
    }

    #[test]
    fn a_nameless_row_still_gets_a_filename() {
        assert_eq!(output_name("", ""), "Resume.md");
        assert_eq!(output_name("", "Engineer"), "Engineer.md");
        assert_eq!(output_name("Acme", ""), "Acme.md");
    }

    #[test]
    fn the_pasted_posting_is_preferred_over_captured_notes() {
        let brief = job_brief("Acme", "Engineer", "Remote", "raw ocr text", "the real posting");
        assert!(brief.contains("the real posting"));
        assert!(!brief.contains("raw ocr text"), "notes are the fallback, not both");
        assert!(brief.contains("Company: Acme"));
        assert!(brief.contains("Location: Remote"));
    }

    #[test]
    fn notes_are_used_when_nothing_was_pasted() {
        let brief = job_brief("Acme", "Engineer", "", "raw ocr text", "");
        assert!(brief.contains("raw ocr text"));
        assert!(!brief.contains("Location:"), "an empty field is left out entirely");
    }

    #[test]
    fn the_prompt_forbids_inventing_things() {
        // The one rule that matters: this output goes out under the
        // candidate's name, and they may not read it closely first.
        assert!(SYSTEM_PROMPT.contains("NEVER invent"));
        assert!(SYSTEM_PROMPT.to_lowercase().contains("do not exaggerate"));
    }

    #[test]
    fn the_master_lives_next_to_the_workbook() {
        let path = master_path(Path::new("/docs/JobApplications.xlsx")).unwrap();
        assert!(path.ends_with("Resume_master.md"));
        assert_eq!(path.parent(), Path::new("/docs/JobApplications.xlsx").parent());
    }
}
