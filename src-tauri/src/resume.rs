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

Return the tailored resume as JSON matching the schema you were given. Bullets are single sentences without a leading dash. Put skills and similar lists in a section's "items"; put jobs, degrees and projects in its "entries"."#;

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

/// Filename stem for a tailored resume, from the role it was aimed at.
/// The extension is added by the caller, which writes more than one file.
///
/// Named after the job rather than the date, because the reason to open one
/// six weeks later is "what did I send Amazon?", and a date answers a
/// question nobody asks.
///
/// The consequence is that saving twice for one job overwrites: the stem
/// is company and position, nothing else. That is the behaviour you want
/// for the common case - tailor, read it, dislike a line, tailor again -
/// where a `-2` file would just be litter you have to tell apart later.
/// Applying to the same role at the same company months later does lose
/// the first PDF, which is the price.
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
    trimmed.trim_matches('-').to_string()
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

/// Pulls the text out of a resume the user already has.
///
/// Everyone applying for jobs has a resume as a PDF or a Word file, and
/// asking them to paste three pages of it into a textarea is the reason
/// they would not bother. Both formats are readable with crates already
/// here - lopdf came in with the PDF writer, and a .docx is a zip of XML.
pub fn import_text(path: &Path) -> Result<String, String> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();

    match extension.as_str() {
        "pdf" => from_pdf(path),
        "docx" => from_docx(path),
        // Tidied like the others: a file exported from somewhere else has
        // the same runs of blank lines, and inconsistency here would show
        // up as "the import worked differently that time".
        "txt" | "md" | "markdown" | "text" => std::fs::read_to_string(path)
            .map(|text| tidy(&text))
            .map_err(|e| format!("Could not read '{}': {e}", path.display())),
        other => Err(format!(
            "Cannot read a '.{other}' file. Use a PDF, a Word .docx, or plain text."
        )),
    }
}

fn from_pdf(path: &Path) -> Result<String, String> {
    let doc = lopdf::Document::load(path)
        .map_err(|e| format!("Could not open that PDF: {e}"))?;
    let pages: Vec<u32> = doc.get_pages().keys().copied().collect();
    if pages.is_empty() {
        return Err("That PDF has no pages.".to_string());
    }
    let text = doc
        .extract_text(&pages)
        .map_err(|e| format!("Could not read text from that PDF: {e}"))?;
    let cleaned = tidy(&text);
    if cleaned.trim().is_empty() {
        return Err(
            "No text found in that PDF - it may be a scan. Paste the text instead."
                .to_string(),
        );
    }
    Ok(cleaned)
}

/// A .docx is a zip; the document body is one XML file inside it.
fn from_docx(path: &Path) -> Result<String, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Could not open '{}': {e}", path.display()))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| format!("That .docx could not be opened: {e}"))?;
    let mut xml = String::new();
    {
        use std::io::Read;
        let mut entry = zip
            .by_name("word/document.xml")
            .map_err(|_| "That file is not a Word document.".to_string())?;
        entry
            .read_to_string(&mut xml)
            .map_err(|e| format!("Could not read the document body: {e}"))?;
    }
    Ok(tidy(&docx_text(&xml)))
}

/// Text runs out of WordprocessingML, with paragraphs kept as line breaks.
///
/// Deliberately not a full XML parse: only two tags matter, and every
/// other element in a .docx is formatting this has no use for.
fn docx_text(xml: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;

    while let Some(start) = rest.find('<') {
        // A paragraph or explicit break ends the line.
        if rest[start..].starts_with("</w:p>") || rest[start..].starts_with("<w:br") {
            out.push('\n');
        }
        // <w:t> and <w:t xml:space="preserve"> both hold text.
        if rest[start..].starts_with("<w:t>") || rest[start..].starts_with("<w:t ") {
            if let Some(open_end) = rest[start..].find('>') {
                let after = &rest[start + open_end + 1..];
                if let Some(close) = after.find("</w:t>") {
                    out.push_str(&unescape_xml(&after[..close]));
                    rest = &after[close..];
                    continue;
                }
            }
        }
        let Some(next) = rest[start + 1..].find('<') else { break };
        rest = &rest[start + 1 + next..];
    }
    out
}

fn unescape_xml(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

/// Collapses the runs of blank lines and stray spaces that extraction
/// leaves behind, so what lands in the box looks like a resume.
fn tidy(text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut blanks = 0;
    for line in text.lines() {
        let trimmed = line.trim_end().to_string();
        if trimmed.trim().is_empty() {
            blanks += 1;
            if blanks > 1 {
                continue;
            }
        } else {
            blanks = 0;
        }
        lines.push(trimmed);
    }
    lines.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filename_says_which_job_it_was_for() {
        assert_eq!(
            output_name("Amazon", "Robotics - Software Development Engineer"),
            "Amazon-Robotics-Software-Development-Engineer"
        );
        assert_eq!(output_name("AtriCure, Inc.", "IT Co-op"), "AtriCure-Inc-IT-Co-op");
    }

    #[test]
    fn characters_a_filesystem_refuses_are_removed() {
        let name = output_name("A/B\\C:D*E?F", "Engineer <Senior>");
        assert!(!name.contains(['/', '\\', ':', '*', '?', '<', '>']));
        assert!(!name.is_empty());
    }

    #[test]
    fn a_very_long_title_does_not_become_a_very_long_filename() {
        let name = output_name("Company", &"Very Long Title ".repeat(20));
        assert!(name.chars().count() <= 84, "got {} chars", name.chars().count());
    }

    #[test]
    fn a_nameless_row_still_gets_a_filename() {
        assert_eq!(output_name("", ""), "Resume");
        assert_eq!(output_name("", "Engineer"), "Engineer");
        assert_eq!(output_name("Acme", ""), "Acme");
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

    #[test]
    fn word_text_runs_become_lines() {
        let xml = concat!(
            "<w:document><w:body>",
            "<w:p><w:r><w:t>Jun Du</w:t></w:r></w:p>",
            "<w:p><w:r><w:t xml:space=\"preserve\">Software </w:t></w:r>",
            "<w:r><w:t>Engineer</w:t></w:r></w:p>",
            "</w:body></w:document>"
        );
        let text = docx_text(xml);
        assert!(text.contains("Jun Du"));
        assert!(
            text.contains("Software Engineer"),
            "runs inside one paragraph join up: {text:?}"
        );
        assert!(
            text.lines().count() >= 2,
            "paragraphs stay on separate lines: {text:?}"
        );
    }

    #[test]
    fn xml_entities_are_turned_back_into_characters() {
        let xml = "<w:p><w:r><w:t>R&amp;D &lt;lead&gt;</w:t></w:r></w:p>";
        assert!(docx_text(xml).contains("R&D <lead>"));
    }

    #[test]
    fn formatting_tags_contribute_no_text() {
        // A real .docx is mostly formatting; none of it belongs in a resume.
        let xml = "<w:p><w:pPr><w:jc w:val=\"center\"/></w:pPr><w:r><w:rPr><w:b/></w:rPr><w:t>Name</w:t></w:r></w:p>";
        assert_eq!(docx_text(xml).trim(), "Name");
    }

    #[test]
    fn runs_of_blank_lines_are_collapsed() {
        // PDF extraction leaves these behind and they make the box look
        // like the import failed.
        assert_eq!(tidy("a\n\n\n\nb"), "a\n\nb");
        assert_eq!(tidy("  \n\na\n  "), "a");
    }

    #[test]
    fn an_unreadable_format_says_which_ones_work() {
        let err = import_text(Path::new("resume.rtf")).unwrap_err();
        assert!(err.contains("PDF"), "should name what does work: {err}");
        assert!(err.contains("docx"));
    }

    #[test]
    fn a_missing_file_is_reported_not_panicked() {
        assert!(import_text(Path::new("no-such-resume.pdf")).is_err());
        assert!(import_text(Path::new("no-such-resume.docx")).is_err());
        assert!(import_text(Path::new("no-such-resume.txt")).is_err());
    }
}
