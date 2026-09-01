//! Free, offline extraction alternative to the Claude API path, backed
//! by a locally installed Tesseract binary.
//!
//! Shells out to the `tesseract` CLI (its TSV output mode gives per-word
//! bounding boxes and confidence), groups words back into paragraph-level
//! text blocks, then guesses which block is the company, position,
//! location, etc. using layout heuristics - the job title is virtually
//! always the single largest text on any job-posting page. Accuracy is
//! well below Claude's actual understanding of the image, so the full
//! raw OCR text is always attached to `notes` for the user to
//! cross-check and correct by hand, matching the app's "never invent,
//! let the user fix it" philosophy.

use crate::models::ExtractedFields;
use regex::Regex;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Words below this OCR confidence (0-100) are dropped as noise before
/// grouping into text blocks.
const MIN_WORD_CONFIDENCE: f32 = 30.0;

pub fn tesseract_available() -> bool {
    Command::new("tesseract")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// One recognized paragraph-level block of text with its bounding-box
/// position/height in the image, used as a font-size proxy for the
/// layout heuristics below. Tesseract already groups wrapped multi-line
/// titles into a single paragraph, which is exactly the unit this
/// heuristic wants.
#[derive(Debug, Clone)]
pub struct OcrLine {
    pub text: String,
    pub top: f32,
    pub height: f32,
}

pub fn run_ocr(image_bytes: &[u8]) -> Result<Vec<OcrLine>, String> {
    if !tesseract_available() {
        return Err(
            "Tesseract isn't installed or isn't on your PATH. Install it with `brew install tesseract` (macOS), your package manager (Linux), or from https://github.com/tesseract-ocr/tesseract (Windows), then try again."
                .to_string(),
        );
    }

    let extension = match image::guess_format(image_bytes) {
        Ok(image::ImageFormat::Jpeg) => "jpg",
        Ok(image::ImageFormat::WebP) => "webp",
        Ok(image::ImageFormat::Gif) => "gif",
        _ => "png",
    };
    let tmp_path = temp_image_path(extension);
    std::fs::write(&tmp_path, image_bytes)
        .map_err(|e| format!("Could not write temporary image for OCR: {e}"))?;

    let result = run_tesseract(&tmp_path);
    let _ = std::fs::remove_file(&tmp_path);
    result
}

fn temp_image_path(extension: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "job-tracker-ocr-{}-{unique}.{extension}",
        std::process::id()
    ))
}

fn run_tesseract(image_path: &std::path::Path) -> Result<Vec<OcrLine>, String> {
    let output = Command::new("tesseract")
        .arg(image_path)
        .arg("stdout")
        .args(["--psm", "6", "tsv"])
        .output()
        .map_err(|e| format!("Could not run tesseract: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Tesseract failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_tesseract_tsv(&stdout))
}

/// Parses `tesseract ... tsv` output into paragraph-level [`OcrLine`]s.
/// Word rows (`level == 5`) are grouped by `(block_num, par_num)`, which
/// keeps a wrapped multi-line title together as one block.
fn parse_tesseract_tsv(tsv: &str) -> Vec<OcrLine> {
    struct Group {
        key: (i64, i64),
        words: Vec<String>,
        left: f32,
        top: f32,
        right: f32,
        bottom: f32,
    }

    let mut groups: Vec<Group> = Vec::new();

    for row in tsv.lines().skip(1) {
        let cols: Vec<&str> = row.split('\t').collect();
        if cols.len() < 12 || cols[0] != "5" {
            continue;
        }
        let conf: f32 = cols[10].parse().unwrap_or(-1.0);
        if conf < MIN_WORD_CONFIDENCE {
            continue;
        }
        let text = cols[11].trim();
        if text.is_empty() {
            continue;
        }
        let (block_num, par_num) = match (cols[2].parse::<i64>(), cols[3].parse::<i64>()) {
            (Ok(b), Ok(p)) => (b, p),
            _ => continue,
        };
        let (left, top, width, height) = match (
            cols[6].parse::<f32>(),
            cols[7].parse::<f32>(),
            cols[8].parse::<f32>(),
            cols[9].parse::<f32>(),
        ) {
            (Ok(l), Ok(t), Ok(w), Ok(h)) => (l, t, w, h),
            _ => continue,
        };

        let key = (block_num, par_num);
        match groups.last_mut() {
            Some(g) if g.key == key => {
                g.words.push(text.to_string());
                g.left = g.left.min(left);
                g.top = g.top.min(top);
                g.right = g.right.max(left + width);
                g.bottom = g.bottom.max(top + height);
            }
            _ => groups.push(Group {
                key,
                words: vec![text.to_string()],
                left,
                top,
                right: left + width,
                bottom: top + height,
            }),
        }
    }

    groups
        .into_iter()
        .map(|g| OcrLine {
            text: g.words.join(" "),
            top: g.top,
            height: g.bottom - g.top,
        })
        .collect()
}

fn regex_cell<'a>(cell: &'a OnceLock<Regex>, pattern: &str) -> &'a Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("static OCR heuristic regex must be valid"))
}

fn location_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    regex_cell(&RE, r"\b[A-Za-z][A-Za-z .]{1,40},\s*[A-Z]{2}\b")
}

fn salary_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    regex_cell(
        &RE,
        r"\$[\d,]+(?:\.\d+)?[Kk]?\s*(?:-|to|–|—)\s*\$?[\d,]+(?:\.\d+)?[Kk]?(?:\s*/\s*(?:yr|hr|hour|year))?",
    )
}

fn job_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    regex_cell(
        &RE,
        r"(?i)(?:job|req(?:uisition)?)\s*(?:id|#)?\s*[:#]?\s*([A-Za-z0-9-]{3,})",
    )
}

fn posted_date_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    regex_cell(&RE, r"(?i)\b\d+\s*(?:day|days|hour|hours|week|weeks|month|months)\s+ago\b")
}

const WORK_TYPE_KEYWORDS: [(&str, &str); 5] = [
    ("remote", "Remote"),
    ("hybrid", "Hybrid"),
    ("on-site", "On-site"),
    ("onsite", "On-site"),
    ("in-office", "On-site"),
];

const EMPLOYMENT_TYPE_KEYWORDS: [(&str, &str); 8] = [
    ("full-time", "Full-time"),
    ("full time", "Full-time"),
    ("part-time", "Part-time"),
    ("part time", "Part-time"),
    ("contract-to-hire", "Contract-to-hire"),
    ("contract", "Contract"),
    ("internship", "Internship"),
    ("co-op", "Internship"),
];

fn find_keyword(text_lower: &str, table: &[(&str, &str)]) -> Option<String> {
    table
        .iter()
        .find(|(needle, _)| text_lower.contains(needle))
        .map(|(_, label)| label.to_string())
}

/// Guesses structured fields from OCR'd lines using layout heuristics:
/// the largest piece of text near the top is almost always the job
/// title, and the line just above it is usually the company name. Never
/// invents a value it isn't reasonably confident about - unmatched
/// fields stay `None`, and the full raw text always goes into `notes`
/// so the user can correct anything by hand.
pub fn guess_fields(lines: &[OcrLine]) -> ExtractedFields {
    let mut sorted: Vec<&OcrLine> = lines.iter().filter(|l| !l.text.trim().is_empty()).collect();
    sorted.sort_by(|a, b| a.top.partial_cmp(&b.top).unwrap_or(std::cmp::Ordering::Equal));

    let full_text = sorted
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let title_index = sorted
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.height.partial_cmp(&b.height).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i);

    let position = title_index.map(|i| sorted[i].text.trim().to_string());

    let company = title_index.and_then(|i| {
        (0..i)
            .rev()
            .map(|j| sorted[j].text.trim())
            .find(|t| !t.is_empty() && t.chars().count() <= 80)
            .map(|t| t.to_string())
    });

    let mut location = None;
    let mut work_type = None;
    let mut employment_type = None;
    let mut salary_range = None;
    let mut job_id = None;
    let mut posted_date = None;

    for (idx, line) in sorted.iter().enumerate() {
        let lower = line.text.to_lowercase();

        if location.is_none() {
            if let Some(m) = location_re().find(&line.text) {
                location = Some(m.as_str().trim().to_string());
            }
        }
        // The title itself is excluded from the work/employment-type scan:
        // titles like "... Fall Intern/Co-op ..." would otherwise spuriously
        // match "Internship" before the real "Full-time"-style badge line
        // is reached.
        let is_title_line = title_index == Some(idx);
        if work_type.is_none() && !is_title_line {
            work_type = find_keyword(&lower, &WORK_TYPE_KEYWORDS);
        }
        if employment_type.is_none() && !is_title_line {
            employment_type = find_keyword(&lower, &EMPLOYMENT_TYPE_KEYWORDS);
        }
        if salary_range.is_none() {
            if let Some(m) = salary_re().find(&line.text) {
                salary_range = Some(m.as_str().trim().to_string());
            }
        }
        if job_id.is_none() {
            if let Some(caps) = job_id_re().captures(&line.text) {
                job_id = caps.get(1).map(|m| m.as_str().to_string());
            }
        }
        if posted_date.is_none() {
            if let Some(m) = posted_date_re().find(&line.text) {
                posted_date = Some(m.as_str().trim().to_string());
            }
        }
    }

    let notes = if full_text.is_empty() {
        None
    } else {
        let truncated: String = full_text.chars().take(4000).collect();
        Some(format!(
            "[Local OCR - best effort, please double-check]\n{truncated}"
        ))
    };

    ExtractedFields {
        company,
        position,
        location,
        work_type,
        employment_type,
        salary_range,
        job_id,
        posted_date,
        url: None,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, top: f32, height: f32) -> OcrLine {
        OcrLine {
            text: text.to_string(),
            top,
            height,
        }
    }

    /// Mirrors the real LinkedIn screenshot used to manually verify this
    /// feature: Amazon / Robotics SDE intern posting in Westboro, WI.
    fn amazon_posting_lines() -> Vec<OcrLine> {
        vec![
            line("Amazon", 20.0, 24.0),
            line(
                "Robotics - Software Development Engineer Fall Intern/Co-op - 2026",
                60.0,
                40.0,
            ),
            line("Westboro, WI · 4 days ago · 37 people clicked apply", 120.0, 18.0),
            line("Promoted by hirer · Responses managed off LinkedIn", 145.0, 16.0),
            line("Full-time", 180.0, 20.0),
        ]
    }

    #[test]
    fn picks_the_largest_line_as_the_position() {
        let fields = guess_fields(&amazon_posting_lines());
        assert_eq!(
            fields.position.as_deref(),
            Some("Robotics - Software Development Engineer Fall Intern/Co-op - 2026")
        );
    }

    #[test]
    fn picks_the_line_above_the_title_as_the_company() {
        let fields = guess_fields(&amazon_posting_lines());
        assert_eq!(fields.company.as_deref(), Some("Amazon"));
    }

    #[test]
    fn extracts_location_from_a_city_state_pattern() {
        let fields = guess_fields(&amazon_posting_lines());
        assert_eq!(fields.location.as_deref(), Some("Westboro, WI"));
    }

    #[test]
    fn extracts_employment_type_keyword() {
        let fields = guess_fields(&amazon_posting_lines());
        assert_eq!(fields.employment_type.as_deref(), Some("Full-time"));
    }

    #[test]
    fn extracts_posted_date_pattern() {
        let fields = guess_fields(&amazon_posting_lines());
        assert_eq!(fields.posted_date.as_deref(), Some("4 days ago"));
    }

    #[test]
    fn never_invents_a_salary_or_job_id_when_absent() {
        let fields = guess_fields(&amazon_posting_lines());
        assert_eq!(fields.salary_range, None);
        assert_eq!(fields.job_id, None);
        assert_eq!(fields.url, None);
    }

    #[test]
    fn notes_always_carries_the_full_raw_text_for_manual_correction() {
        let fields = guess_fields(&amazon_posting_lines());
        let notes = fields.notes.expect("notes should be populated");
        assert!(notes.contains("Amazon"));
        assert!(notes.contains("Westboro, WI"));
        assert!(notes.contains("best effort"));
    }

    #[test]
    fn detects_remote_work_type() {
        let lines = vec![
            line("Globex", 10.0, 20.0),
            line("Staff Engineer", 40.0, 36.0),
            line("Remote · United States", 80.0, 16.0),
        ];
        let fields = guess_fields(&lines);
        assert_eq!(fields.work_type.as_deref(), Some("Remote"));
    }

    #[test]
    fn detects_salary_range() {
        let lines = vec![
            line("Initech", 10.0, 20.0),
            line("Senior Backend Engineer", 40.0, 36.0),
            line("$140,000 - $180,000 a year", 80.0, 16.0),
        ];
        let fields = guess_fields(&lines);
        assert_eq!(fields.salary_range.as_deref(), Some("$140,000 - $180,000"));
    }

    #[test]
    fn detects_job_id() {
        let lines = vec![
            line("Initech", 10.0, 20.0),
            line("Senior Backend Engineer", 40.0, 36.0),
            line("Job ID: R-2026-04421", 80.0, 16.0),
        ];
        let fields = guess_fields(&lines);
        assert_eq!(fields.job_id.as_deref(), Some("R-2026-04421"));
    }

    #[test]
    fn empty_input_produces_all_none_and_no_notes() {
        let fields = guess_fields(&[]);
        assert_eq!(fields.company, None);
        assert_eq!(fields.position, None);
        assert_eq!(fields.notes, None);
    }
}
