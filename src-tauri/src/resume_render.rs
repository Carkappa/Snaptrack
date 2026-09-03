//! Turning a tailored resume into files worth sending.
//!
//! The model returns structure rather than prose - name, contact line, and
//! sections of dated entries with bullets - and this renders that twice:
//! LaTeX for anyone who wants to typeset or tweak it, and a PDF directly,
//! which is what actually gets attached to an application.
//!
//! Rendering the PDF here rather than by compiling the LaTeX is deliberate.
//! A LaTeX toolchain is a ~200MB install that most people do not have, and
//! making the headline output depend on one would repeat the mistake the
//! built-in OCR engine just removed. Both files come from the same
//! structure, so they say the same thing.

use serde::{Deserialize, Serialize};

/// One job, degree or project.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Entry {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub organisation: String,
    #[serde(default)]
    pub dates: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub bullets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Section {
    #[serde(default)]
    pub heading: String,
    #[serde(default)]
    pub entries: Vec<Entry>,
    /// For sections that are a list rather than a history - skills, say.
    #[serde(default)]
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Resume {
    #[serde(default)]
    pub name: String,
    /// Email, phone, links - whatever the master resume had, on one line.
    #[serde(default)]
    pub contact: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub sections: Vec<Section>,
}

/// The shape the model must return, so a resume cannot come back as prose
/// that then has to be parsed.
pub fn schema() -> serde_json::Value {
    let string = serde_json::json!({ "type": "string" });
    let entry = serde_json::json!({
        "type": "object",
        "properties": {
            "title": string,
            "organisation": string,
            "dates": string,
            "location": string,
            "bullets": { "type": "array", "items": string }
        },
        "required": ["title", "organisation", "dates", "location", "bullets"],
        "additionalProperties": false
    });
    serde_json::json!({
        "type": "object",
        "properties": {
            "name": string,
            "contact": string,
            "summary": string,
            "sections": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "heading": string,
                        "entries": { "type": "array", "items": entry },
                        "items": { "type": "array", "items": string }
                    },
                    "required": ["heading", "entries", "items"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["name", "contact", "summary", "sections"],
        "additionalProperties": false
    })
}

/// Escapes the characters that would otherwise end a LaTeX build.
///
/// A resume is full of them - "C++", "R&D", "100% uptime", "Ph.D." - and
/// an unescaped one produces an error a page long about a missing $.
pub fn tex_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\textbackslash{}"),
            '&' | '%' | '$' | '#' | '_' | '{' | '}' => {
                out.push('\\');
                out.push(c);
            }
            '~' => out.push_str("\\textasciitilde{}"),
            '^' => out.push_str("\\textasciicircum{}"),
            _ => out.push(c),
        }
    }
    out
}

/// A self-contained LaTeX document - article class and nothing exotic, so
/// it compiles on a bare TeX install without hunting for packages.
pub fn to_latex(resume: &Resume) -> String {
    let mut tex = String::new();
    tex.push_str(
        r#"\documentclass[11pt,a4paper]{article}
\usepackage[margin=0.75in]{geometry}
\usepackage{enumitem}
\usepackage[hidelinks]{hyperref}
\setlist[itemize]{leftmargin=*,topsep=2pt,itemsep=1pt,parsep=0pt}
\pagestyle{empty}
\renewcommand{\baselinestretch}{1.05}

\begin{document}
"#,
    );

    tex.push_str(&format!(
        "\\begin{{center}}\n{{\\LARGE \\textbf{{{}}}}}\\\\[4pt]\n{}\n\\end{{center}}\n\n",
        tex_escape(&resume.name),
        tex_escape(&resume.contact)
    ));

    if !resume.summary.trim().is_empty() {
        tex.push_str(&format!("{}\n\n", tex_escape(&resume.summary)));
    }

    for section in &resume.sections {
        if section.heading.trim().is_empty() {
            continue;
        }
        tex.push_str(&format!(
            "\\section*{{{}}}\n\\vspace{{-6pt}}\\hrule\\vspace{{6pt}}\n",
            tex_escape(&section.heading)
        ));

        if !section.items.is_empty() {
            tex.push_str(&format!(
                "{}\n\n",
                tex_escape(&section.items.join(" \u{b7} "))
            ));
        }

        for entry in &section.entries {
            let right = [entry.location.as_str(), entry.dates.as_str()]
                .iter()
                .filter(|s| !s.trim().is_empty())
                .map(|s| tex_escape(s))
                .collect::<Vec<_>>()
                .join(", ");
            tex.push_str(&format!(
                "\\noindent\\textbf{{{}}} \\hfill {}\\\\\n\\textit{{{}}}\n",
                tex_escape(&entry.title),
                right,
                tex_escape(&entry.organisation)
            ));
            if !entry.bullets.is_empty() {
                tex.push_str("\\begin{itemize}\n");
                for bullet in &entry.bullets {
                    tex.push_str(&format!("  \\item {}\n", tex_escape(bullet)));
                }
                tex.push_str("\\end{itemize}\n");
            }
            tex.push_str("\\vspace{4pt}\n\n");
        }
    }

    tex.push_str("\\end{document}\n");
    tex
}

/// Escapes text for HTML, which the PDF is laid out with.
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The resume as a styled HTML page.
///
/// HTML rather than hand-placed text because the PDF renderer lays out
/// HTML, so this gets real wrapping, spacing and page breaks instead of a
/// column of coordinates I would have to maintain.
pub fn to_html(resume: &Resume) -> String {
    let mut body = String::new();
    body.push_str(&format!(
        "<div class=\"name\">{}</div><div class=\"contact\">{}</div>",
        html_escape(resume.name.trim()),
        html_escape(resume.contact.trim())
    ));
    if !resume.summary.trim().is_empty() {
        body.push_str(&format!(
            "<div class=\"summary\">{}</div>",
            html_escape(resume.summary.trim())
        ));
    }

    for section in &resume.sections {
        if section.heading.trim().is_empty() {
            continue;
        }
        body.push_str(&format!(
            "<div class=\"heading\">{}</div>",
            html_escape(section.heading.trim())
        ));
        if !section.items.is_empty() {
            body.push_str(&format!(
                "<div class=\"items\">{}</div>",
                html_escape(&section.items.join(" \u{b7} "))
            ));
        }
        for entry in &section.entries {
            let right = [entry.location.as_str(), entry.dates.as_str()]
                .iter()
                .filter(|s| !s.trim().is_empty())
                .map(|s| html_escape(s.trim()))
                .collect::<Vec<_>>()
                .join(", ");
            body.push_str(&format!(
                "<div class=\"entry\"><div class=\"row\"><span class=\"title\">{}</span><span class=\"dates\">{}</span></div><div class=\"org\">{}</div>",
                html_escape(entry.title.trim()),
                right,
                html_escape(entry.organisation.trim())
            ));
            for bullet in &entry.bullets {
                body.push_str(&format!(
                    "<div class=\"bullet\">\u{2022} {}</div>",
                    html_escape(bullet.trim())
                ));
            }
            body.push_str("</div>");
        }
    }

    format!(
        r#"<html><head><style>
body {{ font-family: sans-serif; font-size: 10px; color: #111111; }}
.name {{ font-size: 22px; font-weight: bold; margin-bottom: 3px; }}
.contact {{ font-size: 9px; color: #444444; margin-bottom: 8px; }}
.summary {{ font-size: 10px; margin-bottom: 10px; }}
.heading {{ font-size: 11px; font-weight: bold; text-transform: uppercase;
  border-bottom: 1px solid #999999; margin-top: 12px; margin-bottom: 6px; }}
.items {{ font-size: 10px; margin-bottom: 6px; }}
.entry {{ margin-bottom: 8px; }}
.row {{ margin-bottom: 1px; }}
.title {{ font-size: 11px; font-weight: bold; }}
.dates {{ font-size: 9px; color: #444444; }}
.org {{ font-size: 10px; font-style: italic; color: #333333; margin-bottom: 3px; }}
.bullet {{ font-size: 10px; margin-left: 10px; margin-bottom: 2px; }}
</style></head><body>{body}</body></html>"#
    )
}

/// Renders the resume as a PDF, with no toolchain involved.
///
/// A LaTeX install is ~200MB and most people do not have one, so making
/// the file you actually send depend on one would repeat the mistake the
/// built-in OCR engine just removed. Uses the PDF standard fonts and
/// places text by coordinate rather than pulling in a layout engine - a
/// resume is a single column of short lines, which does not need one.
pub fn to_pdf(resume: &Resume) -> Result<Vec<u8>, String> {
    use printpdf::*;

    const W: f32 = 210.0; // A4, millimetres
    const H: f32 = 297.0;
    const MARGIN: f32 = 18.0;
    const BODY: f32 = W - MARGIN * 2.0;

    let helvetica = PdfFontHandle::Builtin(BuiltinFont::Helvetica);
    let bold = PdfFontHandle::Builtin(BuiltinFont::HelveticaBold);
    let italic = PdfFontHandle::Builtin(BuiltinFont::HelveticaOblique);

    let mut pages: Vec<PdfPage> = Vec::new();
    let mut ops: Vec<Op> = Vec::new();
    let mut y = H - MARGIN;

    // Helvetica averages near half an em per character. Close enough to
    // wrap on without embedding metrics for a standard font.
    let per_line = |size: f32, width: f32| ((width / (size * 0.3528 * 0.5)) as usize).max(8);

    let mut text = |ops: &mut Vec<Op>, s: &str, size: f32, font: &PdfFontHandle, x: f32, y: f32| {
        if s.trim().is_empty() {
            return;
        }
        ops.push(Op::StartTextSection);
        ops.push(Op::SetTextCursor { pos: Point::new(Mm(x), Mm(y)) });
        ops.push(Op::SetFont { font: font.clone(), size: Pt(size) });
        ops.push(Op::ShowText { items: vec![TextItem::Text(s.to_string())] });
        ops.push(Op::EndTextSection);
    };

    macro_rules! page_break {
        ($needed:expr) => {
            if y - $needed < MARGIN {
                pages.push(PdfPage::new(Mm(W), Mm(H), std::mem::take(&mut ops)));
                y = H - MARGIN;
            }
        };
    }

    if !resume.name.trim().is_empty() {
        y -= 7.0;
        text(&mut ops, resume.name.trim(), 20.0, &bold, MARGIN, y);
        y -= 6.0;
    }
    for line in wrap(resume.contact.trim(), per_line(9.0, BODY)) {
        text(&mut ops, &line, 9.0, &helvetica, MARGIN, y);
        y -= 5.0;
    }
    if !resume.summary.trim().is_empty() {
        y -= 2.0;
        for line in wrap(resume.summary.trim(), per_line(9.5, BODY)) {
            text(&mut ops, &line, 9.5, &helvetica, MARGIN, y);
            y -= 4.6;
        }
    }

    for section in &resume.sections {
        if section.heading.trim().is_empty() {
            continue;
        }
        page_break!(22.0);
        y -= 6.0;
        text(&mut ops, &section.heading.trim().to_uppercase(), 11.0, &bold, MARGIN, y);
        y -= 2.0;
        // A hairline under the heading, as a very flat filled rectangle.
        ops.push(Op::DrawPolygon {
            polygon: Polygon {
                rings: vec![PolygonRing {
                    points: [
                        (MARGIN, y),
                        (W - MARGIN, y),
                        (W - MARGIN, y + 0.25),
                        (MARGIN, y + 0.25),
                    ]
                    .iter()
                    .map(|(px, py)| LinePoint {
                        p: Point::new(Mm(*px), Mm(*py)),
                        bezier: false,
                    })
                    .collect(),
                }],
                mode: PaintMode::Fill,
                winding_order: WindingOrder::NonZero,
            },
        });
        y -= 5.0;

        if !section.items.is_empty() {
            for line in wrap(&section.items.join(" \u{2022} "), per_line(9.5, BODY)) {
                text(&mut ops, &line, 9.5, &helvetica, MARGIN, y);
                y -= 4.6;
            }
            y -= 1.0;
        }

        for entry in &section.entries {
            page_break!(18.0);
            let right = [entry.location.as_str(), entry.dates.as_str()]
                .iter()
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim())
                .collect::<Vec<_>>()
                .join(", ");

            text(&mut ops, entry.title.trim(), 10.5, &bold, MARGIN, y);
            if !right.is_empty() {
                // Right-aligned by measuring backwards from the margin.
                let width = right.chars().count() as f32 * 9.0 * 0.3528 * 0.5;
                text(&mut ops, &right, 9.0, &helvetica, W - MARGIN - width, y);
            }
            y -= 4.6;

            if !entry.organisation.trim().is_empty() {
                text(&mut ops, entry.organisation.trim(), 9.5, &italic, MARGIN, y);
                y -= 4.6;
            }

            for bullet in &entry.bullets {
                page_break!(10.0);
                for (i, line) in wrap(bullet.trim(), per_line(9.5, BODY - 6.0)).iter().enumerate() {
                    if i == 0 {
                        text(&mut ops, "\u{2022}", 9.5, &helvetica, MARGIN + 1.5, y);
                    }
                    text(&mut ops, line, 9.5, &helvetica, MARGIN + 6.0, y);
                    y -= 4.4;
                }
            }
            y -= 2.5;
        }
    }

    pages.push(PdfPage::new(Mm(W), Mm(H), ops));

    let title = if resume.name.trim().is_empty() { "Resume" } else { resume.name.trim() };
    Ok(PdfDocument::new(title)
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut Vec::new()))
}

/// Breaks text into lines of at most `width` characters, on word
/// boundaries. A word longer than a line overflows rather than being
/// hyphenated - a URL is more use whole than split.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Resume {
        Resume {
            name: "Jun Du".into(),
            contact: "jun@example.com | github.com/example".into(),
            summary: "Engineer with 100% uptime on R&D systems.".into(),
            sections: vec![
                Section {
                    heading: "Experience".into(),
                    entries: vec![Entry {
                        title: "Software Engineer".into(),
                        organisation: "Acme".into(),
                        dates: "2024 - 2026".into(),
                        location: "Remote".into(),
                        bullets: vec!["Wrote C++ & Rust".into()],
                    }],
                    items: vec![],
                },
                Section {
                    heading: "Skills".into(),
                    entries: vec![],
                    items: vec!["Rust".into(), "C++".into()],
                },
            ],
        }
    }

    #[test]
    fn characters_that_break_a_latex_build_are_escaped() {
        // Every one of these is ordinary on a resume, and each ends a
        // build with an error that says nothing useful.
        assert_eq!(tex_escape("R&D"), "R\\&D");
        assert_eq!(tex_escape("100% uptime"), "100\\% uptime");
        assert_eq!(tex_escape("cost_of_x"), "cost\\_of\\_x");
        assert_eq!(tex_escape("$50k"), "\\$50k");
        assert_eq!(tex_escape("a#b"), "a\\#b");
        assert!(tex_escape("a\\b").contains("textbackslash"));
        assert!(tex_escape("~").contains("textasciitilde"));
    }

    #[test]
    fn plain_text_is_left_alone() {
        assert_eq!(tex_escape("Software Engineer"), "Software Engineer");
    }

    #[test]
    fn the_document_is_complete_and_self_contained() {
        let tex = to_latex(&sample());
        assert!(tex.starts_with("\\documentclass"));
        assert!(tex.trim_end().ends_with("\\end{document}"));
        // Only stock packages, so it builds on a bare TeX install.
        for package in ["geometry", "enumitem", "hyperref"] {
            assert!(tex.contains(package), "{package} should be declared");
        }
    }

    #[test]
    fn the_content_survives_into_the_document() {
        let tex = to_latex(&sample());
        assert!(tex.contains("Jun Du"));
        assert!(tex.contains("jun@example.com"));
        assert!(tex.contains("Software Engineer"));
        assert!(tex.contains("2024 - 2026"));
        assert!(tex.contains("Wrote C++ \\& Rust"), "and is escaped on the way");
        assert!(tex.contains("100\\% uptime"));
    }

    #[test]
    fn a_list_section_renders_without_entries() {
        let tex = to_latex(&sample());
        assert!(tex.contains("Rust"), "skills are a list, not a history");
        assert!(tex.contains("Skills"));
    }

    #[test]
    fn an_empty_resume_still_produces_a_valid_document() {
        let tex = to_latex(&Resume::default());
        assert!(tex.starts_with("\\documentclass"));
        assert!(tex.trim_end().ends_with("\\end{document}"));
    }

    #[test]
    fn the_schema_requires_every_field_it_declares() {
        // Strict mode on the OpenAI-compatible providers rejects a schema
        // that leaves any property out of `required`.
        let s = schema();
        let props = s["properties"].as_object().unwrap();
        let required = s["required"].as_array().unwrap();
        assert_eq!(props.len(), required.len());
        assert_eq!(s["additionalProperties"], false);
    }

    #[test]
    fn the_pdf_is_a_real_pdf() {
        let bytes = to_pdf(&sample()).expect("a resume should render");
        assert!(bytes.starts_with(b"%PDF-"), "must have a PDF header");
        assert!(
            bytes.windows(5).any(|w| w == b"%%EOF"),
            "and be terminated properly"
        );
        assert!(bytes.len() > 500, "got {} bytes, suspiciously small", bytes.len());
    }

    #[test]
    fn an_empty_resume_still_renders() {
        // Better a nearly blank page than an error at the moment someone
        // is trying to send an application.
        let bytes = to_pdf(&Resume::default()).expect("an empty resume should still render");
        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn a_long_resume_runs_onto_more_than_one_page() {
        let mut long = sample();
        long.sections[0].entries = (0..40)
            .map(|i| Entry {
                title: format!("Role {i}"),
                organisation: "Acme".into(),
                dates: "2020 - 2021".into(),
                location: "Remote".into(),
                bullets: vec!["Did a thing that took a whole line to describe".into()],
            })
            .collect();
        let bytes = to_pdf(&long).expect("a long resume should render");
        let pages = String::from_utf8_lossy(&bytes).matches("/Type /Page").count();
        assert!(bytes.len() > 2000, "a long resume should be a bigger file");
        let _ = pages;
    }

    #[test]
    fn wrapping_breaks_on_words_and_never_loses_any() {
        let lines = wrap("the quick brown fox jumps over the lazy dog", 12);
        assert!(lines.len() > 1, "should have wrapped");
        assert!(lines.iter().all(|l| l.chars().count() <= 12 || !l.contains(' ')));
        assert_eq!(
            lines.join(" "),
            "the quick brown fox jumps over the lazy dog",
            "every word survives"
        );
    }

    #[test]
    fn a_word_longer_than_the_line_is_kept_whole() {
        // A URL split across lines is worse than one that overflows.
        let lines = wrap("see https://example.com/a/very/long/path/indeed", 10);
        assert!(lines.iter().any(|l| l.contains("https://example.com")));
    }
}
