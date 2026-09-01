use crate::models::{ExtractedFields, ExtractionResult};

const ANTHROPIC_VERSION: &str = "2023-06-01";
const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-sonnet-4-6";

const SYSTEM_PROMPT: &str = r#"You are extracting structured data from a screenshot of a job posting or job-application confirmation page (e.g. LinkedIn, Greenhouse, Lever, a company careers page).

Return ONLY raw JSON. No markdown code fences, no preamble, no explanation, no trailing commentary - just the JSON object itself.

The JSON object must have exactly these keys:
- company (string or null)
- position (string or null)
- location (string or null)
- work_type (string or null) - e.g. "Remote", "Hybrid", "On-site"
- employment_type (string or null) - e.g. "Full-time", "Part-time", "Contract", "Internship"
- salary_range (string or null)
- job_id (string or null)
- posted_date (string or null)
- url (string or null)
- notes (string or null) - anything else useful you noticed that doesn't fit another field

Rules:
- Use null for any field that is not visible in the screenshot. Never invent, guess, or infer a value that isn't actually shown.
- Do not hallucinate a company or position name if the image is unclear - use null instead.
- Output must be a single valid JSON object and nothing else."#;

fn strip_code_fences(text: &str) -> String {
    let trimmed = text.trim();
    let without_fence = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```JSON"))
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed);
    let without_fence = without_fence.strip_suffix("```").unwrap_or(without_fence);
    without_fence.trim().to_string()
}

#[derive(serde::Serialize)]
struct ImageSource<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    media_type: &'a str,
    data: &'a str,
}

#[derive(serde::Serialize)]
#[serde(tag = "type")]
enum ContentBlock<'a> {
    #[serde(rename = "image")]
    Image { source: ImageSource<'a> },
    #[serde(rename = "text")]
    Text { text: &'a str },
}

#[derive(serde::Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<AnthropicMessage<'a>>,
}

#[derive(serde::Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: Vec<ContentBlock<'a>>,
}

#[derive(serde::Deserialize)]
struct AnthropicResponseContent {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

#[derive(serde::Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicResponseContent>,
}

#[derive(serde::Deserialize)]
struct AnthropicErrorBody {
    error: AnthropicErrorDetail,
}

#[derive(serde::Deserialize)]
struct AnthropicErrorDetail {
    message: String,
}

/// media_type must be one of the types Claude accepts for images:
/// image/png, image/jpeg, image/webp, image/gif.
pub async fn extract_fields_from_image(
    api_key: &str,
    image_base64: &str,
    media_type: &str,
) -> Result<ExtractionResult, String> {
    let request_body = AnthropicRequest {
        model: MODEL,
        max_tokens: 1024,
        system: SYSTEM_PROMPT,
        messages: vec![AnthropicMessage {
            role: "user",
            content: vec![
                ContentBlock::Image {
                    source: ImageSource {
                        kind: "base64",
                        media_type,
                        data: image_base64,
                    },
                },
                ContentBlock::Text {
                    text: "Extract the job posting fields from this screenshot as JSON.",
                },
            ],
        }],
    };

    let client = reqwest::Client::new();
    let response = client
        .post(ANTHROPIC_MESSAGES_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to reach Anthropic API: {e}"))?;

    let status = response.status();
    let body_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read Anthropic API response: {e}"))?;

    if !status.is_success() {
        let message = serde_json::from_str::<AnthropicErrorBody>(&body_text)
            .map(|b| b.error.message)
            .unwrap_or_else(|_| body_text.clone());
        return Err(format!("Anthropic API error ({status}): {message}"));
    }

    let parsed: AnthropicResponse = serde_json::from_str(&body_text)
        .map_err(|e| format!("Unexpected Anthropic API response shape: {e}"))?;

    let raw_text = parsed
        .content
        .into_iter()
        .find(|c| c.kind == "text")
        .and_then(|c| c.text)
        .ok_or_else(|| "Anthropic API returned no text content.".to_string())?;

    let cleaned = strip_code_fences(&raw_text);

    match serde_json::from_str::<ExtractedFields>(&cleaned) {
        Ok(fields) => Ok(ExtractionResult::Parsed { fields }),
        Err(err) => Ok(ExtractionResult::ParseFailed {
            raw_text,
            error: err.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_plain_backtick_fence() {
        let input = "```\n{\"company\":\"Acme\"}\n```";
        assert_eq!(strip_code_fences(input), "{\"company\":\"Acme\"}");
    }

    #[test]
    fn strips_json_labeled_fence() {
        let input = "```json\n{\"company\":\"Acme\"}\n```";
        assert_eq!(strip_code_fences(input), "{\"company\":\"Acme\"}");
    }

    #[test]
    fn leaves_bare_json_untouched() {
        let input = "{\"company\":\"Acme\"}";
        assert_eq!(strip_code_fences(input), "{\"company\":\"Acme\"}");
    }

    #[test]
    fn parses_valid_extracted_json() {
        let json = r#"{"company":"Acme","position":"Engineer","location":null,
            "work_type":"Remote","employment_type":"Full-time","salary_range":null,
            "job_id":"123","posted_date":null,"url":null,"notes":null}"#;
        let fields: ExtractedFields = serde_json::from_str(json).unwrap();
        assert_eq!(fields.company.as_deref(), Some("Acme"));
        assert_eq!(fields.location, None);
    }

    #[test]
    fn parse_failure_is_reported_not_panicked() {
        let bad = "not json at all";
        let result = serde_json::from_str::<ExtractedFields>(bad);
        assert!(result.is_err());
    }
}
