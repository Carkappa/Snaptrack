use serde::{Deserialize, Serialize};

/// The fixed set of statuses shown in the status dropdown, both in the
/// app's UI and as the Excel data-validation list on the Status column.
pub const STATUSES: [&str; 6] = [
    "Applied",
    "Interviewing",
    "Offered",
    "Rejected",
    "Ghosted",
    "Withdrawn",
];

/// A status the user can apply to an application.
///
/// `kind` is what keeps the response-rate figure meaningful once the list is
/// editable. The app cannot guess whether a status someone invented ("Phone
/// screen", "Take-home") means the employer replied, so it is recorded
/// rather than inferred from the name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusKind {
    /// Sent, nothing back yet. Counts against the response rate.
    Waiting,
    /// They answered - an interview, an offer, or a rejection. A rejection
    /// is still an answer.
    Replied,
    /// Ended by the user. Excluded from the response rate entirely, since
    /// nobody is waiting on a reply.
    Closed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusDef {
    pub name: String,
    pub kind: StatusKind,
}

impl StatusDef {
    pub fn new(name: &str, kind: StatusKind) -> Self {
        Self {
            name: name.to_string(),
            kind,
        }
    }
}

/// The list a fresh install starts from.
pub fn default_status_defs() -> Vec<StatusDef> {
    vec![
        StatusDef::new("Applied", StatusKind::Waiting),
        StatusDef::new("Interviewing", StatusKind::Replied),
        StatusDef::new("Offered", StatusKind::Replied),
        StatusDef::new("Rejected", StatusKind::Replied),
        StatusDef::new("Ghosted", StatusKind::Waiting),
        StatusDef::new("Withdrawn", StatusKind::Closed),
    ]
}

/// Normalises a user-edited list: trims, drops blanks, removes
/// case-insensitive duplicates, and refuses to end up with nothing.
pub fn sanitize_status_defs(defs: Vec<StatusDef>) -> Result<Vec<StatusDef>, String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for def in defs {
        let name = def.name.trim().to_string();
        if name.is_empty() {
            continue;
        }
        if name.chars().count() > 40 {
            return Err(format!(
                "'{name}' is too long for a status (40 characters max)."
            ));
        }
        if !seen.insert(name.to_lowercase()) {
            continue;
        }
        out.push(StatusDef {
            name,
            kind: def.kind,
        });
    }
    if out.is_empty() {
        return Err("Keep at least one status.".to_string());
    }
    Ok(out)
}

#[cfg(test)]
mod status_tests {
    use super::*;

    #[test]
    fn defaults_describe_the_six_built_in_statuses() {
        let defs = default_status_defs();
        assert_eq!(defs.len(), 6);
        assert_eq!(defs[0].name, "Applied");
        assert_eq!(defs[0].kind, StatusKind::Waiting);
        assert_eq!(
            defs.iter().filter(|d| d.kind == StatusKind::Replied).count(),
            3,
            "interviewing, offered and rejected all count as a reply"
        );
        assert_eq!(
            defs.iter().find(|d| d.name == "Withdrawn").unwrap().kind,
            StatusKind::Closed
        );
    }

    #[test]
    fn sanitize_trims_and_drops_blanks() {
        let out = sanitize_status_defs(vec![
            StatusDef::new("  Applied  ", StatusKind::Waiting),
            StatusDef::new("   ", StatusKind::Waiting),
            StatusDef::new("", StatusKind::Replied),
        ])
        .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Applied");
    }

    #[test]
    fn sanitize_drops_case_insensitive_duplicates_keeping_the_first() {
        let out = sanitize_status_defs(vec![
            StatusDef::new("Applied", StatusKind::Waiting),
            StatusDef::new("applied", StatusKind::Replied),
        ])
        .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].kind, StatusKind::Waiting);
    }

    #[test]
    fn sanitize_refuses_an_empty_list_and_over_long_names() {
        assert!(sanitize_status_defs(vec![]).is_err());
        assert!(sanitize_status_defs(vec![StatusDef::new("   ", StatusKind::Waiting)]).is_err());
        let long = "x".repeat(41);
        assert!(sanitize_status_defs(vec![StatusDef::new(&long, StatusKind::Waiting)]).is_err());
    }
}

/// A single row in the job-applications workbook.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobApplication {
    pub date_applied: String,
    pub company: String,
    pub position: String,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub work_type: Option<String>,
    #[serde(default)]
    pub employment_type: Option<String>,
    #[serde(default)]
    pub salary_range: Option<String>,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub last_updated: String,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_status() -> String {
    "Applied".to_string()
}

impl JobApplication {
    /// Case-insensitive, trimmed match used for duplicate detection.
    pub fn dedupe_key(&self) -> (String, String) {
        (
            self.company.trim().to_lowercase(),
            self.position.trim().to_lowercase(),
        )
    }
}

/// Raw fields extracted from a screenshot by the Anthropic API.
/// Every field is optional because the model is instructed to
/// return null for anything it cannot see, never invent a value.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractedFields {
    pub company: Option<String>,
    pub position: Option<String>,
    pub location: Option<String>,
    pub work_type: Option<String>,
    pub employment_type: Option<String>,
    pub salary_range: Option<String>,
    pub job_id: Option<String>,
    pub posted_date: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
}

/// Result of an extraction attempt: either clean structured fields,
/// or the raw text so the user can fix it by hand if parsing failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ExtractionResult {
    Parsed { fields: ExtractedFields },
    ParseFailed { raw_text: String, error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome")]
pub enum SaveResult {
    Saved,
    Duplicate { existing_status: String },
}
