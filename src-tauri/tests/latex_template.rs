//! The LaTeX style path against a resume template shaped like the ones
//! people actually use.
//!
//! The unit tests in the module use a deliberately small document. This
//! one is modelled on the widely-copied "Jake's Resume" layout - custom
//! `\resume*` macros, a `tabular*` inside each entry, `titleformat`, a
//! redefined `\section` - because that shape is what the validator has to
//! accept, and a validator that only accepts toys would send everyone
//! back to the built-in style.

use job_tracker_lib::latex_template;

const REAL_TEMPLATE: &str = r#"\documentclass[letterpaper,11pt]{article}

\usepackage{latexsym}
\usepackage[empty]{fullpage}
\usepackage{titlesec}
\usepackage[usenames,dvipsnames]{color}
\usepackage{enumitem}
\usepackage[hidelinks]{hyperref}
\usepackage{fancyhdr}
\usepackage[english]{babel}
\usepackage{tabularx}

\pagestyle{fancy}
\fancyhf{}
\setlength{\footskip}{5pt}
\addtolength{\oddsidemargin}{-0.5in}
\addtolength{\textwidth}{1in}
\urlstyle{same}
\raggedbottom
\raggedright
\setlength{\tabcolsep}{0in}

\titleformat{\section}{\vspace{-4pt}\scshape\raggedright\large}{}{0em}{}[\color{black}\titlerule \vspace{-5pt}]

\newcommand{\resumeItem}[1]{\item\small{{#1 \vspace{-2pt}}}}
\newcommand{\resumeSubheading}[4]{
  \vspace{-2pt}\item
    \begin{tabular*}{0.97\textwidth}[t]{l@{\extracolsep{\fill}}r}
      \textbf{#1} & #2 \\
      \textit{\small#3} & \textit{\small #4} \\
    \end{tabular*}\vspace{-7pt}
}
\newcommand{\resumeSubItem}[1]{\resumeItem{#1}\vspace{-4pt}}
\renewcommand\labelitemii{$\vcenter{\hbox{\tiny$\bullet$}}$}
\newcommand{\resumeSubHeadingListStart}{\begin{itemize}[leftmargin=0.15in, label={}]}
\newcommand{\resumeSubHeadingListEnd}{\end{itemize}}
\newcommand{\resumeItemListStart}{\begin{itemize}}
\newcommand{\resumeItemListEnd}{\end{itemize}\vspace{-5pt}}

\begin{document}

\begin{center}
    \textbf{\Huge \scshape Jun Du} \\ \vspace{1pt}
    \small 555-0100 $|$ \href{mailto:jun@example.com}{\underline{jun@example.com}}
\end{center}

\section{Education}
  \resumeSubHeadingListStart
    \resumeSubheading
      {Texas A\&M University}{College Station, TX}
      {B.S. Computer Science}{Aug. 2023 -- May 2027}
  \resumeSubHeadingListEnd

\section{Experience}
  \resumeSubHeadingListStart
    \resumeSubheading
      {Software Engineering Intern}{Summer 2025}
      {Acme Robotics}{Remote}
      \resumeItemListStart
        \resumeItem{Cut build times by 40\% by caching intermediate artifacts}
        \resumeItem{Wrote the migration that moved 12 services off the old queue}
      \resumeItemListEnd
  \resumeSubHeadingListEnd

\section{Technical Skills}
 \begin{itemize}[leftmargin=0.15in, label={}]
    \small{\item{
     \textbf{Languages}{: Rust, Python, TypeScript} \\
    }}
 \end{itemize}

\end{document}
"#;

#[test]
fn a_real_template_splits_with_its_style_intact() {
    let t = latex_template::split(REAL_TEMPLATE).expect("should split");

    // Everything that makes it look the way it does has to survive.
    for needed in [
        "\\documentclass[letterpaper,11pt]{article}",
        "\\usepackage{titlesec}",
        "\\titleformat{\\section}",
        "\\newcommand{\\resumeSubheading}[4]",
        "\\addtolength{\\textwidth}{1in}",
    ] {
        assert!(t.preamble.contains(needed), "preamble lost {needed}");
    }
    assert!(!t.body.contains("\\usepackage"), "body should carry no packages");
    assert!(t.body.contains("Acme Robotics"));
}

#[test]
fn every_macro_the_template_defines_is_found() {
    let t = latex_template::split(REAL_TEMPLATE).unwrap();
    let defined = latex_template::commands_defined_in(&t.preamble);
    for macro_name in [
        "resumeItem",
        "resumeSubheading",
        "resumeSubItem",
        "resumeSubHeadingListStart",
        "resumeSubHeadingListEnd",
        "resumeItemListStart",
        "resumeItemListEnd",
    ] {
        assert!(defined.contains(macro_name), "did not find {macro_name} in {defined:?}");
    }
}

#[test]
fn a_body_written_in_the_templates_own_idiom_is_accepted() {
    let t = latex_template::split(REAL_TEMPLATE).unwrap();
    // What a model should produce: same macros, different words.
    let body = r#"\begin{center}
    \textbf{\Huge \scshape Jun Du} \\ \vspace{1pt}
    \small 555-0100 $|$ \href{mailto:jun@example.com}{\underline{jun@example.com}}
\end{center}

\section{Experience}
  \resumeSubHeadingListStart
    \resumeSubheading
      {Backend Engineer}{2026}
      {Stripe}{Remote}
      \resumeItemListStart
        \resumeItem{Built the ledger reconciliation job}
      \resumeItemListEnd
  \resumeSubHeadingListEnd
"#;
    assert_eq!(latex_template::validate(body, &t), Ok(()));

    let out = latex_template::assemble(&t, body);
    assert!(out.contains("\\newcommand{\\resumeSubheading}[4]"), "style kept");
    assert!(out.contains("Stripe"), "new words present");
    assert!(!out.contains("Acme Robotics"), "old words gone");
    assert!(out.trim_end().ends_with("\\end{document}"));
}

#[test]
fn a_body_that_reaches_for_another_template_is_refused() {
    let t = latex_template::split(REAL_TEMPLATE).unwrap();
    // \cventry belongs to moderncv, not to this document. Writing it would
    // produce a .tex that fails to compile with an error naming a macro
    // the author never heard of.
    let err = latex_template::validate("\\cventry{2026}{Engineer}{Stripe}{}{}{}", &t).unwrap_err();
    assert!(err.contains("cventry"), "{err}");
}

#[test]
fn the_words_come_out_readable_for_the_model_to_tailor() {
    let t = latex_template::split(REAL_TEMPLATE).unwrap();
    let text = latex_template::to_plain_text(&t.body);

    for word in [
        "Jun Du",
        "Texas A&M University",
        "Software Engineering Intern",
        "Acme Robotics",
        "Cut build times by 40%",
        "Rust, Python, TypeScript",
    ] {
        assert!(text.contains(word), "plain text lost {word:?}:\n{text}");
    }
    // And none of the machinery should survive into it.
    for machinery in ["resumeSubheading", "tabular", "vspace", "textbf"] {
        assert!(!text.contains(machinery), "plain text kept {machinery}:\n{text}");
    }
}
