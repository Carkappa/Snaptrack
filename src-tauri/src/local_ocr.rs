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

/// Mean luminance (0-255) below which the image is treated as dark-mode
/// and inverted. Tesseract is trained on dark text over a light page; a
/// dark-mode screenshot is the exact inverse and reads badly without this.
const DARK_MODE_THRESHOLD: f32 = 110.0;

/// Text in a UI screenshot is typically 12-16px, where Tesseract wants
/// roughly 30px cap height. Anything narrower than this gets scaled up.
const UPSCALE_BELOW_WIDTH: u32 = 1600;

/// Ceiling on the scaled result, so a 4K screenshot isn't blown up into
/// something that takes seconds to OCR for no gain.
const MAX_SCALED_WIDTH: u32 = 3600;

/// Page-segmentation modes tried in order, best-scoring result wins.
/// 6 assumes one uniform block, which a job page with a sidebar is not;
/// 3 is full auto page segmentation and 11 is sparse text, and which one
/// wins genuinely varies by site.
const PSM_MODES: [&str; 3] = ["6", "3", "11"];

/// How much to enlarge an image before OCR. Returns 1.0 when the image is
/// already big enough, and never enlarges past `MAX_SCALED_WIDTH`.
fn scale_factor(width: u32, height: u32) -> f32 {
    if width == 0 || height == 0 || width >= UPSCALE_BELOW_WIDTH {
        return 1.0;
    }
    let wanted = 2.0_f32;
    let capped = MAX_SCALED_WIDTH as f32 / width as f32;
    wanted.min(capped).max(1.0)
}

/// True when the image is mostly dark, i.e. light text on a dark ground.
fn is_dark(gray: &image::GrayImage) -> bool {
    if gray.is_empty() {
        return false;
    }
    let total: u64 = gray.pixels().map(|p| p.0[0] as u64).sum();
    let mean = total as f32 / gray.len() as f32;
    mean < DARK_MODE_THRESHOLD
}

/// Stretches the grey range to full black-to-white. UI screenshots often
/// use mid-grey text on an off-white card, which OCRs worse than the same
/// text at full contrast.
fn stretch_contrast(gray: &mut image::GrayImage) {
    let (mut lo, mut hi) = (255u8, 0u8);
    for p in gray.pixels() {
        lo = lo.min(p.0[0]);
        hi = hi.max(p.0[0]);
    }
    if hi <= lo || (hi - lo) > 220 {
        return; // already full-range, or a blank image
    }
    let span = (hi - lo) as f32;
    for p in gray.pixels_mut() {
        let v = ((p.0[0] - lo) as f32 / span * 255.0).round().clamp(0.0, 255.0);
        p.0[0] = v as u8;
    }
}

/// Grayscale, invert if dark-mode, stretch contrast, and upscale small
/// images. Tesseract gets the raw screenshot otherwise, and these three
/// things are where most of its accuracy on UI captures comes from.
pub fn preprocess(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let decoded = image::load_from_memory(image_bytes)
        .map_err(|e| format!("Could not read that image: {e}"))?;
    let mut gray = decoded.to_luma8();

    if is_dark(&gray) {
        image::imageops::invert(&mut gray);
    }
    stretch_contrast(&mut gray);

    let (w, h) = gray.dimensions();
    let factor = scale_factor(w, h);
    let scaled = if factor > 1.0 {
        image::imageops::resize(
            &gray,
            (w as f32 * factor) as u32,
            (h as f32 * factor) as u32,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        gray
    };

    let mut png = Vec::new();
    image::DynamicImage::ImageLuma8(scaled)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| format!("Could not encode the preprocessed image: {e}"))?;
    Ok(png)
}

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
    /// The individual text lines this block was built from, with each
    /// line's height. Tesseract often puts a company name and the job
    /// title in one paragraph; the only thing separating them is that the
    /// company is set smaller, which is lost once the block is flattened.
    pub sub_lines: Vec<SubLine>,
}

#[derive(Debug, Clone)]
pub struct SubLine {
    pub text: String,
    pub height: f32,
}

impl OcrLine {
    /// Convenience for tests and callers that don't care about sub-lines.
    pub fn flat(text: &str, top: f32, height: f32) -> Self {
        Self {
            text: text.to_string(),
            top,
            height,
            sub_lines: vec![SubLine {
                text: text.to_string(),
                height,
            }],
        }
    }
}

/// How big the text in a block actually is.
///
/// Not the same as the block's bounding box, which is what this used to
/// compare: a row of chips with rounded borders spans a tall box while its
/// text is small, and a title that wrapped onto two lines has a box twice
/// its type size. Comparing boxes made "On-site" in a pill outrank the job
/// title. The tallest line inside a block is the thing that tracks type
/// size, so that is what the title heuristic ranks by.
fn type_size(block: &OcrLine) -> f32 {
    let tallest = block
        .sub_lines
        .iter()
        .fold(0.0_f32, |acc, l| acc.max(l.height));
    if tallest > 0.0 {
        tallest
    } else {
        block.height
    }
}

/// A first line noticeably smaller than the rest of its block is a
/// different thing from what follows - on every job board that means the
/// company sitting above the title.
const COMPANY_LINE_RATIO: f32 = 0.75;

/// Splits a block whose first line is set smaller than the rest into
/// (company, title). Returns None when the block is all one size, which is
/// the normal case for a title that simply wrapped.
fn split_company_from_title(block: &OcrLine) -> Option<(String, String)> {
    if block.sub_lines.len() < 2 {
        return None;
    }
    let first = &block.sub_lines[0];
    let rest = &block.sub_lines[1..];
    let rest_max = rest.iter().fold(0.0_f32, |acc, l| acc.max(l.height));
    if rest_max <= 0.0 || first.height >= rest_max * COMPANY_LINE_RATIO {
        return None;
    }
    let company = first.text.trim();
    let title = rest
        .iter()
        .map(|l| l.text.trim())
        .collect::<Vec<_>>()
        .join(" ");
    if company.is_empty() || title.is_empty() || company.chars().count() > 80 {
        return None;
    }
    Some((company.to_string(), title))
}

/// Short suffixes that really are part of a company name, so they survive
/// the noise strip below.
const COMPANY_SUFFIXES: [&str; 10] = [
    "co", "inc", "llc", "ltd", "plc", "ag", "sa", "bv", "nv", "gmbh",
];

/// Removes OCR debris from the edges of a short value.
///
/// Icons and badges next to a company or location get read as stray
/// tokens - a tick becomes "cee", a pin becomes "Y" - and they end up
/// glued to the value. A leading single character is never the start of a
/// real name, and a trailing lower-case fragment is not a company suffix
/// unless it is one of the handful that exist.
fn strip_edge_noise(value: &str) -> String {
    let mut tokens: Vec<&str> = value.split_whitespace().collect();

    while let Some(first) = tokens.first() {
        let bare = first.trim_matches(|c: char| !c.is_alphanumeric());
        if tokens.len() > 1 && bare.chars().count() <= 1 {
            tokens.remove(0);
        } else {
            break;
        }
    }

    while let Some(last) = tokens.last() {
        let bare: String = last
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        let is_noise = tokens.len() > 1
            && bare.chars().count() <= 3
            && !bare.is_empty()
            && bare.chars().all(|c| c.is_ascii_lowercase())
            && !COMPANY_SUFFIXES.contains(&bare.as_str())
            && last.chars().all(|c| !c.is_ascii_uppercase());
        if is_noise {
            tokens.pop();
        } else {
            break;
        }
    }

    tokens.join(" ").trim().to_string()
}

pub fn run_ocr(image_bytes: &[u8]) -> Result<Vec<OcrLine>, String> {
    if !tesseract_available() {
        return Err(
            "Tesseract isn't installed or isn't on your PATH. Install it with `brew install tesseract` (macOS), your package manager (Linux), or from https://github.com/tesseract-ocr/tesseract (Windows), then try again."
                .to_string(),
        );
    }

    // Preprocessing is where most of the accuracy on UI screenshots comes
    // from. If it fails for any reason, fall back to the raw bytes rather
    // than refusing to OCR at all.
    let (bytes, extension) = match preprocess(image_bytes) {
        Ok(png) => (png, "png"),
        Err(_) => (
            image_bytes.to_vec(),
            match image::guess_format(image_bytes) {
                Ok(image::ImageFormat::Jpeg) => "jpg",
                Ok(image::ImageFormat::WebP) => "webp",
                Ok(image::ImageFormat::Gif) => "gif",
                _ => "png",
            },
        ),
    };

    let tmp_path = temp_image_path(extension);
    std::fs::write(&tmp_path, &bytes)
        .map_err(|e| format!("Could not write temporary image for OCR: {e}"))?;

    let result = run_tesseract_best_of(&tmp_path);
    let _ = std::fs::remove_file(&tmp_path);
    result
}

/// Runs Tesseract in each page-segmentation mode and keeps whichever read
/// the page best.
///
/// Which mode wins genuinely varies: a single-column posting does well
/// under 6, a page with a sidebar under 3, a sparse confirmation screen
/// under 11. Guessing one for everybody leaves accuracy on the table, and
/// the runs are fast enough that trying three is not noticeable.
fn run_tesseract_best_of(image_path: &std::path::Path) -> Result<Vec<OcrLine>, String> {
    let mut best: Option<(f32, Vec<OcrLine>)> = None;
    let mut last_error = None;

    for psm in PSM_MODES {
        match run_tesseract(image_path, psm) {
            Ok(lines) => {
                let score = reading_score(&lines);
                if best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
                    best = Some((score, lines));
                }
            }
            Err(e) => last_error = Some(e),
        }
    }

    match best {
        Some((_, lines)) => Ok(lines),
        None => Err(last_error.unwrap_or_else(|| "Tesseract produced no output.".to_string())),
    }
}

/// How much readable text a run produced. Total recognised characters
/// across blocks - a mode that mis-segments the page returns fewer, shorter
/// blocks, so this separates them without needing a second confidence pass.
fn reading_score(lines: &[OcrLine]) -> f32 {
    lines
        .iter()
        .map(|l| l.text.chars().filter(|c| !c.is_whitespace()).count() as f32)
        .sum()
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

fn run_tesseract(image_path: &std::path::Path, psm: &str) -> Result<Vec<OcrLine>, String> {
    let output = Command::new("tesseract")
        .arg(image_path)
        .arg("stdout")
        .args(["--psm", psm, "tsv"])
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
        /// Words kept per text line, so a block's internal type sizes
        /// survive into `sub_lines`.
        lines: Vec<(i64, Vec<String>, f32)>,
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
        let line_num = cols[4].parse::<i64>().unwrap_or(0);
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
                match g.lines.last_mut() {
                    Some((n, words, h)) if *n == line_num => {
                        words.push(text.to_string());
                        *h = h.max(height);
                    }
                    _ => g.lines.push((line_num, vec![text.to_string()], height)),
                }
            }
            _ => groups.push(Group {
                key,
                words: vec![text.to_string()],
                left,
                top,
                right: left + width,
                bottom: top + height,
                lines: vec![(line_num, vec![text.to_string()], height)],
            }),
        }
    }

    groups
        .into_iter()
        .map(|g| OcrLine {
            text: g.words.join(" "),
            top: g.top,
            height: g.bottom - g.top,
            sub_lines: g
                .lines
                .into_iter()
                .map(|(_, words, height)| SubLine {
                    text: words.join(" "),
                    height,
                })
                .collect(),
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

const WORK_TYPE_KEYWORDS: [(&str, &str); 8] = [
    ("remote", "Remote"),
    ("hybrid", "Hybrid"),
    ("on-site", "On-site"),
    ("on site", "On-site"),
    ("onsite", "On-site"),
    ("in-office", "On-site"),
    ("in office", "On-site"),
    ("in-person", "On-site"),
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

    // The title is the largest *text* on the page. Blocks that carry no
    // real words - a chip border read as ") )", stray punctuation - are not
    // candidates however big their box is.
    let title_index = sorted
        .iter()
        .enumerate()
        .filter(|(_, l)| l.text.chars().filter(|c| c.is_alphanumeric()).count() >= 3)
        .max_by(|(_, a), (_, b)| {
            type_size(a)
                .partial_cmp(&type_size(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i);

    // Tesseract often folds the company and the title into one paragraph.
    // When it has, the company is the smaller first line of that block
    // rather than a block above it, and reading it the old way leaves the
    // company empty and the title carrying the company name.
    let split = title_index.and_then(|i| split_company_from_title(sorted[i]));

    let position = match &split {
        Some((_, title)) => Some(title.clone()),
        None => title_index.map(|i| sorted[i].text.trim().to_string()),
    };

    let company = match &split {
        Some((company, _)) => Some(company.clone()),
        None => title_index.and_then(|i| {
            (0..i)
                .rev()
                .map(|j| sorted[j].text.trim())
                .find(|t| !t.is_empty() && t.chars().count() <= 80)
                .map(|t| t.to_string())
        }),
    };

    let company = company
        .map(|c| strip_edge_noise(&c))
        .filter(|c| !c.is_empty());

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
                let cleaned = strip_edge_noise(m.as_str().trim());
                if !cleaned.is_empty() {
                    location = Some(cleaned);
                }
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
        OcrLine::flat(text, top, height)
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

    // ---- preprocessing ----

    fn solid(w: u32, h: u32, level: u8) -> image::GrayImage {
        image::GrayImage::from_pixel(w, h, image::Luma([level]))
    }

    #[test]
    fn a_dark_screenshot_is_recognised_as_dark() {
        assert!(is_dark(&solid(10, 10, 20)), "near-black is dark mode");
        assert!(is_dark(&solid(10, 10, 100)), "dark grey is dark mode");
        assert!(!is_dark(&solid(10, 10, 200)), "a light page is not");
        assert!(!is_dark(&solid(10, 10, 255)), "white is not");
    }

    #[test]
    fn contrast_stretch_pulls_a_flat_range_to_full_black_and_white() {
        // Mid-grey text on an off-white card: the exact case that OCRs badly.
        let mut img = image::GrayImage::from_fn(4, 1, |x, _| {
            image::Luma([[120u8, 140, 160, 180][x as usize]])
        });
        stretch_contrast(&mut img);
        let out: Vec<u8> = img.pixels().map(|p| p.0[0]).collect();
        assert_eq!(out[0], 0, "the darkest pixel becomes black");
        assert_eq!(out[3], 255, "the lightest becomes white");
        assert!(out[1] < out[2], "ordering is preserved");
    }

    #[test]
    fn contrast_stretch_leaves_a_full_range_image_alone() {
        let mut img = image::GrayImage::from_fn(2, 1, |x, _| {
            image::Luma([if x == 0 { 0u8 } else { 255 }])
        });
        stretch_contrast(&mut img);
        assert_eq!(img.get_pixel(0, 0).0[0], 0);
        assert_eq!(img.get_pixel(1, 0).0[0], 255);
    }

    #[test]
    fn contrast_stretch_does_not_divide_by_zero_on_a_blank_image() {
        let mut img = solid(4, 4, 128);
        stretch_contrast(&mut img);
        assert!(img.pixels().all(|p| p.0[0] == 128), "a flat image is left as-is");
    }

    #[test]
    fn small_screenshots_are_enlarged_and_large_ones_are_not() {
        assert_eq!(scale_factor(800, 600), 2.0, "UI text this small needs upscaling");
        assert_eq!(scale_factor(1599, 900), 2.0);
        assert_eq!(scale_factor(1600, 900), 1.0, "already big enough");
        assert_eq!(scale_factor(3840, 2160), 1.0, "a 4K capture is left alone");
    }

    #[test]
    fn upscaling_is_capped_so_a_wide_image_is_not_blown_up() {
        // 2x would exceed the ceiling, so the factor is reduced to fit.
        let f = scale_factor(1900, 100);
        assert_eq!(f, 1.0, "already at or past the upscale threshold");
        let f = scale_factor(1500, 100);
        assert!(f * 1500.0 <= MAX_SCALED_WIDTH as f32 + 1.0, "never past the ceiling");
    }

    #[test]
    fn scale_factor_never_shrinks_or_divides_by_zero() {
        assert_eq!(scale_factor(0, 0), 1.0);
        assert_eq!(scale_factor(0, 100), 1.0);
        assert!(scale_factor(10, 10) >= 1.0, "preprocessing must never shrink text");
    }

    #[test]
    fn preprocess_returns_a_png_and_inverts_a_dark_capture() {
        let mut dark = Vec::new();
        image::DynamicImage::ImageLuma8(solid(40, 20, 15))
            .write_to(&mut std::io::Cursor::new(&mut dark), image::ImageFormat::Png)
            .unwrap();

        let out = preprocess(&dark).expect("a valid image should preprocess");
        assert_eq!(
            image::guess_format(&out).unwrap(),
            image::ImageFormat::Png,
            "Tesseract is handed a PNG whatever came in"
        );
        let img = image::load_from_memory(&out).unwrap().to_luma8();
        assert!(
            img.get_pixel(0, 0).0[0] > 200,
            "a dark-mode capture comes out light so Tesseract can read it"
        );
        let (w, _) = img.dimensions();
        assert!(w > 40, "a small capture is enlarged, got width {w}");
    }

    #[test]
    fn preprocess_reports_rather_than_panics_on_junk() {
        assert!(preprocess(b"not an image at all").is_err());
        assert!(preprocess(&[]).is_err());
    }

    // ---- page-segmentation selection ----

    #[test]
    fn the_run_that_read_the_most_text_wins() {
        let sparse = vec![OcrLine::flat("Acme", 0.0, 10.0)];
        let full = vec![
            OcrLine::flat("Acme Corporation", 0.0, 20.0),
            OcrLine::flat("Senior Engineer", 30.0, 14.0),
        ];
        assert!(
            reading_score(&full) > reading_score(&sparse),
            "a mode that segmented the page properly scores higher"
        );
    }

    #[test]
    fn whitespace_does_not_inflate_the_score() {
        let padded = vec![OcrLine::flat("a b", 0.0, 1.0)];
        let solid_text = vec![OcrLine::flat("abc", 0.0, 1.0)];
        assert!(reading_score(&solid_text) > reading_score(&padded));
    }

    #[test]
    fn an_empty_read_scores_zero() {
        assert_eq!(reading_score(&[]), 0.0);
    }

    #[test]
    fn every_segmentation_mode_is_tried() {
        assert_eq!(PSM_MODES.len(), 3);
        assert!(PSM_MODES.contains(&"6"), "single uniform block");
        assert!(PSM_MODES.contains(&"3"), "auto page segmentation");
        assert!(PSM_MODES.contains(&"11"), "sparse text");
    }

    // ---- the real LinkedIn card that came back wrong ----

    /// Reproduces what Tesseract actually produced for the Amazon posting:
    /// the company, the title, the location line and the metadata all
    /// folded into a single paragraph, with the logo tick read as "cee"
    /// and the location pin as "Y". The company came back empty and the
    /// title carried "Amazon cee" on the front.
    fn merged_amazon_block() -> Vec<OcrLine> {
        vec![
            OcrLine {
                text: "Amazon cee Robotics - Software Development Engineer Fall Intern/Co-op - 2026 Y Westboro, WI - 5 days ago - 42 people clicked apply".into(),
                top: 10.0,
                height: 90.0,
                sub_lines: vec![
                    SubLine { text: "Amazon cee".into(), height: 12.0 },
                    SubLine { text: "Robotics - Software Development Engineer Fall".into(), height: 21.0 },
                    SubLine { text: "Intern/Co-op - 2026".into(), height: 21.0 },
                    SubLine { text: "Y Westboro, WI - 5 days ago - 42 people clicked apply".into(), height: 11.0 },
                ],
            },
            OcrLine::flat("On-site Full-time", 120.0, 12.0),
        ]
    }

    #[test]
    fn a_merged_company_and_title_block_is_split() {
        let f = guess_fields(&merged_amazon_block());
        assert_eq!(
            f.company.as_deref(),
            Some("Amazon"),
            "the company is the smaller first line of the merged block"
        );
        let position = f.position.unwrap();
        assert!(
            position.starts_with("Robotics - Software Development Engineer"),
            "the title must not carry the company on the front, got {position:?}"
        );
        assert!(
            !position.contains("Amazon"),
            "the company must not remain in the title, got {position:?}"
        );
    }

    #[test]
    fn icon_debris_is_stripped_from_the_company_and_location() {
        let f = guess_fields(&merged_amazon_block());
        assert_eq!(f.company.as_deref(), Some("Amazon"), "the tick read as 'cee' is dropped");
        assert_eq!(
            f.location.as_deref(),
            Some("Westboro, WI"),
            "the pin read as 'Y' is dropped from the front of the location"
        );
    }

    #[test]
    fn the_chips_beside_the_title_are_read() {
        let f = guess_fields(&merged_amazon_block());
        assert_eq!(f.work_type.as_deref(), Some("On-site"));
        assert_eq!(f.employment_type.as_deref(), Some("Full-time"));
    }

    // ---- the splitting rule itself ----

    #[test]
    fn a_title_that_merely_wrapped_is_not_split() {
        // Every line the same size: one long title, no company in it.
        let block = OcrLine {
            text: "Senior Software Development Engineer Fall Intern".into(),
            top: 0.0,
            height: 40.0,
            sub_lines: vec![
                SubLine { text: "Senior Software Development".into(), height: 20.0 },
                SubLine { text: "Engineer Fall Intern".into(), height: 20.0 },
            ],
        };
        assert!(
            split_company_from_title(&block).is_none(),
            "a wrapped title must stay whole"
        );
    }

    #[test]
    fn a_single_line_block_is_never_split() {
        assert!(split_company_from_title(&OcrLine::flat("Acme", 0.0, 10.0)).is_none());
    }

    #[test]
    fn a_first_line_only_slightly_smaller_is_not_a_company() {
        let block = OcrLine {
            text: "a b".into(),
            top: 0.0,
            height: 30.0,
            sub_lines: vec![
                SubLine { text: "Almost the same size".into(), height: 19.0 },
                SubLine { text: "as the line below it".into(), height: 20.0 },
            ],
        };
        assert!(split_company_from_title(&block).is_none());
    }

    // ---- edge-noise stripping ----

    #[test]
    fn strips_a_stray_leading_character() {
        assert_eq!(strip_edge_noise("Y Westboro, WI"), "Westboro, WI");
        assert_eq!(strip_edge_noise("© Remote"), "Remote");
        assert_eq!(strip_edge_noise("9 San Francisco, CA"), "San Francisco, CA");
    }

    #[test]
    fn strips_a_trailing_lowercase_fragment() {
        assert_eq!(strip_edge_noise("Amazon cee"), "Amazon");
        assert_eq!(strip_edge_noise("Stripe wv"), "Stripe");
    }

    #[test]
    fn keeps_real_company_suffixes_and_state_codes() {
        assert_eq!(strip_edge_noise("Acme Inc"), "Acme Inc");
        assert_eq!(strip_edge_noise("Widgets Co"), "Widgets Co");
        assert_eq!(strip_edge_noise("Foo GmbH"), "Foo GmbH");
        assert_eq!(
            strip_edge_noise("Westboro, WI"),
            "Westboro, WI",
            "a two-letter state code is not debris"
        );
    }

    #[test]
    fn never_strips_a_value_down_to_nothing() {
        assert_eq!(strip_edge_noise("Y"), "Y");
        assert_eq!(strip_edge_noise("cee"), "cee");
        assert_eq!(strip_edge_noise(""), "");
        assert_eq!(strip_edge_noise("IBM"), "IBM");
        assert_eq!(strip_edge_noise("X Corp"), "Corp");
    }

    // ---- sub-lines survive parsing ----

    #[test]
    fn the_parser_keeps_each_lines_height() {
        // level, page, block, par, line, word, left, top, width, height, conf, text
        let tsv = "level	page	block	par	line	word	left	top	width	height	conf	text
5	1	1	1	1	1	10	10	50	12	95	Amazon
5	1	1	1	2	1	10	30	200	22	95	Robotics
5	1	1	1	2	2	220	30	60	22	95	Engineer
";
        let lines = parse_tesseract_tsv(tsv);
        assert_eq!(lines.len(), 1, "one paragraph");
        assert_eq!(lines[0].sub_lines.len(), 2, "two text lines inside it");
        assert_eq!(lines[0].sub_lines[0].text, "Amazon");
        assert_eq!(lines[0].sub_lines[0].height, 12.0);
        assert_eq!(lines[0].sub_lines[1].text, "Robotics Engineer");
        assert_eq!(lines[0].sub_lines[1].height, 22.0);

        let (company, title) = split_company_from_title(&lines[0]).expect("should split");
        assert_eq!(company, "Amazon");
        assert_eq!(title, "Robotics Engineer");
    }

    /// The same LinkedIn card once preprocessing separated the blocks
    /// properly. The trap here is the chip row: rounded pill borders make
    /// its bounding box taller than the title's, while its text is small.
    /// Ranking by box size picked it as the job title.
    fn separated_amazon_blocks() -> Vec<OcrLine> {
        vec![
            OcrLine::flat("Amazon", 10.0, 12.0),
            OcrLine {
                text: "Robotics - Software Development Engineer Fall Intern/Co-op - 2026".into(),
                top: 30.0,
                height: 44.0, // two wrapped lines
                sub_lines: vec![
                    SubLine { text: "Robotics - Software Development Engineer Fall".into(), height: 21.0 },
                    SubLine { text: "Intern/Co-op - 2026".into(), height: 21.0 },
                ],
            },
            OcrLine::flat("Westboro, WI - 5 days ago - 42 people clicked apply", 80.0, 11.0),
            OcrLine::flat("Promoted by hirer - Responses managed off LinkedIn", 100.0, 11.0),
            OcrLine {
                // A pill's border spans far more than its text.
                text: "On-site ) ) Full-time".into(),
                top: 120.0,
                height: 46.0,
                sub_lines: vec![SubLine { text: "On-site ) ) Full-time".into(), height: 12.0 }],
            },
        ]
    }

    #[test]
    fn a_chip_row_does_not_outrank_the_job_title() {
        let f = guess_fields(&separated_amazon_blocks());
        let position = f.position.expect("a position should be found");
        assert!(
            position.starts_with("Robotics - Software Development Engineer"),
            "the title must win over a chip whose box is taller, got {position:?}"
        );
        assert_eq!(
            f.company.as_deref(),
            Some("Amazon"),
            "with the right title, the company is the block above it"
        );
    }

    #[test]
    fn the_chips_are_read_as_chips_not_as_the_title() {
        let f = guess_fields(&separated_amazon_blocks());
        assert_eq!(f.work_type.as_deref(), Some("On-site"));
        assert_eq!(
            f.employment_type.as_deref(),
            Some("Full-time"),
            "Intern/Co-op in the title must not win over the actual badge"
        );
        assert_eq!(f.location.as_deref(), Some("Westboro, WI"));
    }

    #[test]
    fn type_size_measures_text_not_the_bounding_box() {
        let wrapped_title = OcrLine {
            text: "two lines".into(),
            top: 0.0,
            height: 44.0,
            sub_lines: vec![
                SubLine { text: "line one".into(), height: 21.0 },
                SubLine { text: "line two".into(), height: 21.0 },
            ],
        };
        let tall_chip = OcrLine {
            text: "On-site".into(),
            top: 0.0,
            height: 46.0,
            sub_lines: vec![SubLine { text: "On-site".into(), height: 12.0 }],
        };
        assert!(
            tall_chip.height > wrapped_title.height,
            "the chip's box really is taller - that was the trap"
        );
        assert!(
            type_size(&wrapped_title) > type_size(&tall_chip),
            "but its text is smaller, which is what should decide"
        );
    }

    #[test]
    fn a_block_of_punctuation_is_never_the_title() {
        let lines = vec![
            OcrLine::flat("Acme", 0.0, 10.0),
            OcrLine::flat("Senior Engineer", 20.0, 18.0),
            OcrLine {
                text: ") ) (".into(),
                top: 50.0,
                height: 60.0,
                sub_lines: vec![SubLine { text: ") ) (".into(), height: 40.0 }],
            },
        ];
        assert_eq!(
            guess_fields(&lines).position.as_deref(),
            Some("Senior Engineer"),
            "border artifacts carry no words and cannot be a title"
        );
    }

    #[test]
    fn type_size_falls_back_to_the_box_when_there_are_no_sub_lines() {
        let bare = OcrLine { text: "x".into(), top: 0.0, height: 17.0, sub_lines: vec![] };
        assert_eq!(type_size(&bare), 17.0);
    }
}
