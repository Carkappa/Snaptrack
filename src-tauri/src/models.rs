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
