//! Keeping someone else's LaTeX resume style while changing the words.
//!
//! People who have a LaTeX resume have usually spent real time on it, and
//! the built-in template throws all of that away. This keeps it, by way of
//! one rule: **the preamble is never touched**.
//!
//! Everything that makes a resume look the way it does - the document
//! class, the packages, the fonts, the colours, the spacing, and every
//! custom macro like `\resumeSubheading` - lives above `\begin{document}`.
//! Copy that through byte for byte and the style cannot drift. Only the
//! body is rewritten, and it is rewritten using the macros the original
//! body already called, so the new text goes through the same formatting
//! the old text did.
//!
//! The model writes that body, which means it can get it wrong, and a
//! `.tex` that does not compile is a worse outcome than a plain one. So
//! the body is checked before it is written: balanced braces, matched
//! environments, nothing that belongs in a preamble, and no command that
//! is not either defined in the preamble, already used in the original
//! body, or a plain LaTeX primitive. A body that fails is refused and the
//! caller falls back to the built-in template.

use std::collections::BTreeSet;

/// A resume `.tex` split into the part that carries the style and the part
/// that carries the words.
#[derive(Debug, Clone, PartialEq)]
pub struct LatexTemplate {
    /// Everything up to and including `\begin{document}`.
    pub preamble: String,
    /// Everything between the document markers - the words.
    pub body: String,
    /// Everything from `\end{document}` on. Usually just that line.
    pub tail: String,
}

const BEGIN_DOCUMENT: &str = "\\begin{document}";
const END_DOCUMENT: &str = "\\end{document}";

/// Whether a file looks like a LaTeX document rather than prose that
/// happens to contain a backslash.
pub fn looks_like_latex(source: &str) -> bool {
    source.contains(BEGIN_DOCUMENT) && source.contains("\\documentclass")
}

/// Splits a `.tex` file into preamble, body and tail.
pub fn split(source: &str) -> Result<LatexTemplate, String> {
    let begin = source.find(BEGIN_DOCUMENT).ok_or_else(|| {
        "That .tex file has no \\begin{document}, so there is no style to keep.".to_string()
    })?;
    let end = source.rfind(END_DOCUMENT).ok_or_else(|| {
        "That .tex file has no \\end{document} - it looks truncated.".to_string()
    })?;
    if end < begin {
        return Err("That .tex file has \\end{document} before \\begin{document}.".to_string());
    }

    let body_start = begin + BEGIN_DOCUMENT.len();
    Ok(LatexTemplate {
        preamble: source[..body_start].to_string(),
        body: source[body_start..end].to_string(),
        tail: source[end..].to_string(),
    })
}

/// Puts a new body back between the original preamble and tail.
pub fn assemble(template: &LatexTemplate, body: &str) -> String {
    format!(
        "{}\n{}\n{}",
        template.preamble.trim_end(),
        body.trim(),
        template.tail.trim_start()
    )
}

/// Commands used in a chunk of LaTeX, without the leading backslash.
///
/// Deliberately includes commands that appear in comments: this feeds a
/// permissiveness check, and being too generous about what the original
/// document used is safer than refusing a body that would have compiled.
pub fn commands_in(source: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\\' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < chars.len() && (chars[j].is_ascii_alphabetic() || chars[j] == '@') {
            j += 1;
        }
        if j > i + 1 {
            found.insert(chars[i + 1..j].iter().collect::<String>());
            i = j;
        } else {
            // An escape such as \& or \\ - not a command name.
            i += 2;
        }
    }
    found
}

/// Commands the preamble defines, so a body may call them.
///
/// Covers the four ways a resume template usually declares one, including
/// `\def`, which plenty of older templates still use.
pub fn commands_defined_in(preamble: &str) -> BTreeSet<String> {
    let mut defined = BTreeSet::new();
    for declarer in [
        "\\newcommand",
        "\\renewcommand",
        "\\providecommand",
        "\\DeclareRobustCommand",
        "\\def",
        "\\let",
        "\\newenvironment",
        "\\renewenvironment",
    ] {
        let mut from = 0;
        while let Some(at) = preamble[from..].find(declarer) {
            let after = from + at + declarer.len();
            from = after;
            // \newcommand{\foo} and \newcommand\foo are both legal, as is
            // \newcommand*{\foo}.
            let rest = preamble[after..].trim_start_matches(['*', '{', ' ']);
            let name: String = rest
                .strip_prefix('\\')
                .unwrap_or("")
                .chars()
                .take_while(|c| c.is_ascii_alphabetic() || *c == '@')
                .collect();
            if !name.is_empty() {
                defined.insert(name);
            }
            // An environment also brings \begin{name} into play, which the
            // environment check handles separately.
            if declarer.contains("environment") {
                let env: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '*')
                    .collect();
                if !env.is_empty() {
                    defined.insert(env);
                }
            }
        }
    }
    defined
}

/// Environments opened in a chunk, in the order they are opened.
fn environments_in(source: &str, marker: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = source[from..].find(marker) {
        let after = from + at + marker.len();
        from = after;
        let name: String = source[after..]
            .chars()
            .take_while(|c| *c != '}')
            .collect::<String>()
            .trim()
            .to_string();
        if !name.is_empty() {
            found.push(name);
        }
    }
    found
}

/// Commands any LaTeX document can use without anything declaring them.
///
/// Not exhaustive, and does not need to be: an unrecognised command is
/// only rejected when the original body did not use it either, so this
/// list only has to cover what a model might reasonably reach for on its
/// own.
const PRIMITIVES: &[&str] = &[
    "begin", "end", "item", "textbf", "textit", "texttt", "textsc", "emph", "underline",
    "textsuperscript", "textsubscript", "large", "Large", "LARGE", "huge", "Huge", "small",
    "footnotesize", "scriptsize", "tiny", "normalsize", "bfseries", "itshape", "rmfamily",
    "sffamily", "ttfamily", "centering", "raggedright", "raggedleft", "hfill", "vfill",
    "hspace", "vspace", "smallskip", "medskip", "bigskip", "newline", "linebreak", "par",
    "noindent", "indent", "quad", "qquad", "hrule", "hrulefill", "rule", "href", "url",
    "section", "subsection", "subsubsection", "paragraph", "textrm", "text", "and",
    "today", "space", "null", "leavevmode", "makebox", "parbox", "mbox", "phantom",
    "color", "textcolor", "label", "ref", "vrule", "strut", "relax", "protect",
];

/// Checks a model-written body against the document it has to live in.
pub fn validate(body: &str, template: &LatexTemplate) -> Result<(), String> {
    let stripped = strip_comments(body);

    // A stray brace is the failure that produces the most baffling LaTeX
    // error, so it is worth catching here where the message can be plain.
    let mut depth = 0i32;
    let chars: Vec<char> = stripped.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 1, // whatever follows is escaped, not a brace
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth < 0 {
                    return Err("the body closes a brace it never opened".to_string());
                }
            }
            _ => {}
        }
        i += 1;
    }
    if depth != 0 {
        return Err(format!("the body leaves {depth} brace(s) open"));
    }

    for forbidden in [
        "\\documentclass",
        "\\usepackage",
        BEGIN_DOCUMENT,
        END_DOCUMENT,
    ] {
        if stripped.contains(forbidden) {
            return Err(format!(
                "the body contains {forbidden}, which belongs to the preamble this is keeping"
            ));
        }
    }

    // Environments have to nest, or the document ends somewhere other than
    // where it looks like it does.
    let mut open: Vec<String> = Vec::new();
    let mut cursor = 0usize;
    loop {
        let next_begin = stripped[cursor..].find("\\begin{").map(|p| cursor + p);
        let next_end = stripped[cursor..].find("\\end{").map(|p| cursor + p);
        let (at, is_begin) = match (next_begin, next_end) {
            (Some(b), Some(e)) => {
                if b < e {
                    (b, true)
                } else {
                    (e, false)
                }
            }
            (Some(b), None) => (b, true),
            (None, Some(e)) => (e, false),
            (None, None) => break,
        };
        let marker_len = if is_begin { 7 } else { 5 };
        let name: String = stripped[at + marker_len..]
            .chars()
            .take_while(|c| *c != '}')
            .collect();
        if is_begin {
            open.push(name);
        } else {
            match open.pop() {
                Some(expected) if expected == name => {}
                Some(expected) => {
                    return Err(format!(
                        "the body closes {name} while {expected} is still open"
                    ))
                }
                None => return Err(format!("the body ends {name} without beginning it")),
            }
        }
        cursor = at + marker_len;
    }
    if let Some(unclosed) = open.first() {
        return Err(format!("the body never closes {unclosed}"));
    }

    // Anything the original document already used is fair game, as is
    // anything its preamble defines. Anything else would not compile.
    let mut allowed: BTreeSet<String> = PRIMITIVES.iter().map(|s| s.to_string()).collect();
    allowed.extend(commands_in(&template.body));
    allowed.extend(commands_in(&template.preamble));
    allowed.extend(commands_defined_in(&template.preamble));

    let used = commands_in(&stripped);
    let unknown: Vec<&String> = used.iter().filter(|c| !allowed.contains(*c)).collect();
    if let Some(first) = unknown.first() {
        return Err(format!(
            "the body calls \\{first}, which your document does not define"
        ));
    }

    // Environments the original never used will not exist either.
    let mut allowed_envs: BTreeSet<String> = environments_in(&template.body, "\\begin{")
        .into_iter()
        .collect();
    allowed_envs.extend(environments_in(&template.preamble, "\\newenvironment{"));
    for standard in ["itemize", "enumerate", "description", "center", "tabular", "flushleft"] {
        allowed_envs.insert(standard.to_string());
    }
    for env in environments_in(&stripped, "\\begin{") {
        if !allowed_envs.contains(&env) {
            return Err(format!(
                "the body opens a {env} environment, which your document does not define"
            ));
        }
    }

    if stripped.trim().is_empty() {
        return Err("the body is empty".to_string());
    }
    Ok(())
}

/// Drops `%` comments, keeping escaped `\%`.
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    for line in source.split('\n') {
        let mut kept = String::new();
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() {
                kept.push(chars[i]);
                kept.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if chars[i] == '%' {
                break;
            }
            kept.push(chars[i]);
            i += 1;
        }
        out.push_str(&kept);
        out.push('\n');
    }
    out
}

/// Readable text from a LaTeX body, for the model to tailor from.
///
/// Not a LaTeX interpreter and does not need to be - it produces the
/// master resume's *words*, which is what the tailoring step reads. The
/// formatting is preserved separately, by keeping the document itself.
pub fn to_plain_text(body: &str) -> String {
    let source = strip_comments(body);
    let mut out = String::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] != '\\' {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        // An escaped literal: \& \% \_ \# \$ \{ \}
        if i + 1 < chars.len() && !chars[i + 1].is_ascii_alphabetic() {
            let next = chars[i + 1];
            if next == '\\' {
                out.push('\n');
            } else if "&%_#${}".contains(next) {
                out.push(next);
            }
            i += 2;
            continue;
        }

        let mut j = i + 1;
        while j < chars.len() && chars[j].is_ascii_alphabetic() {
            j += 1;
        }
        let name: String = chars[i + 1..j].iter().collect();
        i = j;

        match name.as_str() {
            "item" => out.push_str("\n- "),
            "begin" | "end" => {
                // Skip the environment name; the content stays.
                if i < chars.len() && chars[i] == '{' {
                    while i < chars.len() && chars[i] != '}' {
                        i += 1;
                    }
                    i += 1;
                }
                out.push('\n');
            }
            _ => {
                // Optional arguments are formatting, not words.
                while i < chars.len() && chars[i] == '[' {
                    while i < chars.len() && chars[i] != ']' {
                        i += 1;
                    }
                    i += 1;
                }
                // Braced arguments are usually the words themselves
                // (\textbf{Amazon}), so they are kept and separated - a
                // macro's arguments are normally distinct fields.
                let mut kept_any = false;
                while i < chars.len() && chars[i] == '{' {
                    let mut depth = 0;
                    let start = i;
                    while i < chars.len() {
                        match chars[i] {
                            '\\' => i += 1,
                            '{' => depth += 1,
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                    let inner: String = chars[start + 1..i.min(chars.len())].iter().collect();
                    if kept_any {
                        out.push_str(" \u{b7} ");
                    }
                    out.push_str(&to_plain_text(&inner));
                    kept_any = true;
                    i += 1;
                }
                if !kept_any {
                    out.push(' ');
                }
            }
        }
    }

    // Runs of blank lines and trailing spaces are an artefact of stripping,
    // not something the author wrote.
    let mut lines: Vec<String> = Vec::new();
    let mut blank = false;
    for line in out.split('\n') {
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            if !blank && !lines.is_empty() {
                lines.push(String::new());
            }
            blank = true;
        } else {
            lines.push(trimmed);
            blank = false;
        }
    }
    while lines.last().map(|l| l.is_empty()).unwrap_or(false) {
        lines.pop();
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"\documentclass[letterpaper,11pt]{article}
\usepackage{titlesec}
\newcommand{\resumeItem}[1]{\item\small{#1}}
\newcommand{\resumeSubheading}[4]{\textbf{#1} & #2 \\ \textit{#3} & \textit{#4}}

\begin{document}
\begin{center}\textbf{\Huge Jun Du} \\ jun@example.com\end{center}
\section{Experience}
\resumeSubheading{Software Engineer}{2024 - 2026}{Acme}{Remote}
\begin{itemize}
  \resumeItem{Built a job tracker}
\end{itemize}
\end{document}
"#;

    #[test]
    fn a_resume_tex_is_recognised_and_a_plain_file_is_not() {
        assert!(looks_like_latex(SAMPLE));
        assert!(!looks_like_latex("Jun Du\nSoftware engineer\nC:\\path\\thing"));
    }

    #[test]
    fn splitting_keeps_the_preamble_byte_for_byte() {
        let t = split(SAMPLE).unwrap();
        assert!(t.preamble.contains("\\documentclass[letterpaper,11pt]{article}"));
        assert!(t.preamble.contains("\\newcommand{\\resumeSubheading}"));
        assert!(t.preamble.ends_with(BEGIN_DOCUMENT));
        assert!(t.body.contains("Built a job tracker"));
        assert!(!t.body.contains("\\documentclass"));
        assert!(t.tail.starts_with(END_DOCUMENT));
    }

    #[test]
    fn a_file_with_no_document_is_refused_with_a_reason() {
        let err = split("\\documentclass{article}\nnothing else").unwrap_err();
        assert!(err.contains("begin{document}"), "{err}");
    }

    #[test]
    fn assembling_round_trips_a_body_back_into_the_document() {
        let t = split(SAMPLE).unwrap();
        let rebuilt = assemble(&t, "\\section{New}\nWords");
        assert!(rebuilt.contains("\\newcommand{\\resumeSubheading}"));
        assert!(rebuilt.contains("\\section{New}"));
        assert!(rebuilt.trim_end().ends_with(END_DOCUMENT));
        assert!(!rebuilt.contains("Built a job tracker"));
    }

    #[test]
    fn the_preambles_own_macros_are_found() {
        let defined = commands_defined_in(&split(SAMPLE).unwrap().preamble);
        assert!(defined.contains("resumeItem"), "{defined:?}");
        assert!(defined.contains("resumeSubheading"), "{defined:?}");
    }

    #[test]
    fn a_body_reusing_the_documents_own_macros_is_accepted() {
        let t = split(SAMPLE).unwrap();
        let body = "\\section{Experience}\n\\resumeSubheading{Engineer}{2025}{Globex}{Austin}\n\\begin{itemize}\\resumeItem{Shipped it}\\end{itemize}";
        assert_eq!(validate(body, &t), Ok(()));
    }

    #[test]
    fn a_body_inventing_a_macro_is_refused_by_name() {
        let t = split(SAMPLE).unwrap();
        let err = validate("\\fancyHeading{Nope}", &t).unwrap_err();
        assert!(err.contains("fancyHeading"), "{err}");
    }

    #[test]
    fn unbalanced_braces_are_caught_before_the_file_is_written() {
        let t = split(SAMPLE).unwrap();
        assert!(validate("\\textbf{unclosed", &t).unwrap_err().contains("open"));
        assert!(validate("closed} too many", &t)
            .unwrap_err()
            .contains("never opened"));
    }

    #[test]
    fn an_escaped_brace_is_not_counted_as_a_brace() {
        let t = split(SAMPLE).unwrap();
        assert_eq!(validate("100\\% of \\{literal\\} text", &t), Ok(()));
    }

    #[test]
    fn mismatched_environments_are_caught() {
        let t = split(SAMPLE).unwrap();
        assert!(validate("\\begin{itemize}\\end{center}", &t)
            .unwrap_err()
            .contains("while"));
        assert!(validate("\\begin{itemize}", &t)
            .unwrap_err()
            .contains("never closes"));
    }

    #[test]
    fn a_body_that_redeclares_the_document_is_refused() {
        let t = split(SAMPLE).unwrap();
        for bad in [
            "\\documentclass{article}",
            "\\usepackage{xcolor}",
            "\\begin{document}x",
        ] {
            assert!(validate(bad, &t).is_err(), "{bad} should be refused");
        }
    }

    #[test]
    fn a_commented_out_command_does_not_count_against_the_body() {
        let t = split(SAMPLE).unwrap();
        assert_eq!(validate("Words % \\notARealCommand{x}", &t), Ok(()));
    }

    #[test]
    fn plain_text_keeps_the_words_and_drops_the_formatting() {
        let text = to_plain_text(&split(SAMPLE).unwrap().body);
        assert!(text.contains("Jun Du"), "{text}");
        assert!(text.contains("jun@example.com"), "{text}");
        assert!(text.contains("Software Engineer"), "{text}");
        assert!(text.contains("Built a job tracker"), "{text}");
        assert!(!text.contains("\\resumeItem"), "{text}");
        assert!(!text.contains("itemize"), "{text}");
    }

    #[test]
    fn plain_text_unescapes_the_characters_latex_makes_you_escape() {
        assert_eq!(to_plain_text("Research \\& Development"), "Research & Development");
        assert_eq!(to_plain_text("Grew 40\\%"), "Grew 40%");
    }

    #[test]
    fn an_empty_body_is_not_worth_writing() {
        let t = split(SAMPLE).unwrap();
        assert!(validate("   \n  ", &t).unwrap_err().contains("empty"));
    }
}
