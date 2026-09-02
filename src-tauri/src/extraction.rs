use crate::models::{ExtractedFields, ExtractionResult};

const ANTHROPIC_VERSION: &str = "2023-06-01";
const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";

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

const OPENAI_URL: &str = "https://api.openai.com/v1/chat/completions";
const GEMINI_URL_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

const USER_PROMPT: &str = "Extract the job posting fields from this screenshot as JSON.";

const TOOL_NAME: &str = "record_job_posting";

/// The shape every provider is held to.
///
/// Asking for JSON in prose and hoping is what the ParseFailed path exists
/// for; all three providers can enforce a schema instead, which removes the
/// failure rather than handling it. Every field is nullable on purpose - a
/// field that isn't visible must come back null, never invented.
fn field_schema() -> serde_json::Value {
    let nullable_string = serde_json::json!({ "type": ["string", "null"] });
    let mut properties = serde_json::Map::new();
    for key in [
        "company",
        "position",
        "location",
        "work_type",
        "employment_type",
        "salary_range",
        "job_id",
        "posted_date",
        "url",
        "notes",
    ] {
        properties.insert(key.to_string(), nullable_string.clone());
    }
    let required: Vec<&str> = properties.keys().map(|k| k.as_str()).collect();
    serde_json::json!({
        "type": "object",
        "properties": properties,
        // OpenAI's strict mode requires every property be listed as required;
        // the null union above is what makes "not visible" expressible.
        "required": required,
        "additionalProperties": false
    })
}

/// Each provider wants the same three things - a system prompt, an image, and
/// a question - in its own shape. These build the body; the caller sends it.
/// Keeping them pure means the wire format is covered by tests without a
/// network call or an API key.
fn claude_body(model: &str, image_base64: &str, media_type: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "max_tokens": 1024,
        "system": SYSTEM_PROMPT,
        // A forced tool call is how Anthropic guarantees a shape: the model
        // must call this tool, and its input is validated against the schema.
        "tools": [{
            "name": TOOL_NAME,
            "description": "Record the fields visible in a job-posting screenshot.",
            "input_schema": field_schema()
        }],
        "tool_choice": { "type": "tool", "name": TOOL_NAME },
        "messages": [{
            "role": "user",
            "content": [
                { "type": "image", "source": {
                    "type": "base64", "media_type": media_type, "data": image_base64 } },
                { "type": "text", "text": USER_PROMPT }
            ]
        }]
    })
}

fn openai_body(model: &str, image_base64: &str, media_type: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "max_tokens": 1024,
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": TOOL_NAME,
                "strict": true,
                "schema": field_schema()
            }
        },
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": [
                { "type": "text", "text": USER_PROMPT },
                { "type": "image_url", "image_url": {
                    "url": format!("data:{media_type};base64,{image_base64}") } }
            ]}
        ]
    })
}

/// `model` is unused here on purpose: Gemini names the model in the URL
/// path rather than the request body. Kept in the signature so all three
/// builders are called the same way.
fn gemini_body(_model: &str, image_base64: &str, media_type: &str) -> serde_json::Value {
    serde_json::json!({
        "systemInstruction": { "parts": [{ "text": SYSTEM_PROMPT }] },
        "contents": [{
            "role": "user",
            "parts": [
                { "text": USER_PROMPT },
                { "inlineData": { "mimeType": media_type, "data": image_base64 } }
            ]
        }],
        "generationConfig": {
            "maxOutputTokens": 1024,
            "responseMimeType": "application/json",
            "responseSchema": gemini_schema()
        }
    })
}

/// Ollama takes the OCR text rather than the image.
///
/// Tesseract has already done the hard part - turning pixels into words -
/// so the local model only has to work out which words are the company and
/// which are the title. That is language, not layout, so it generalises to
/// boards no heuristic was ever written for, and a 3B text model does it on
/// a CPU in a second or two where a vision model would need a GPU.
fn ollama_body(model: &str, ocr_text: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "stream": false,
        // Ollama enforces a JSON schema the same way the cloud providers do.
        "format": field_schema(),
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": format!("{USER_PROMPT}

{ocr_text}") }
        ]
    })
}

/// Turns text already read off a screenshot into fields, using a model
/// running on this machine. No key, no network beyond localhost.
pub async fn extract_fields_from_text(
    host: &str,
    model: &str,
    ocr_text: &str,
) -> Result<ExtractionResult, String> {
    let url = format!("{}/api/chat", host.trim_end_matches('/'));
    let response = reqwest::Client::new()
        .post(&url)
        .header("content-type", "application/json")
        .json(&ollama_body(model, ocr_text))
        .send()
        .await
        .map_err(|e| {
            format!("Couldn't reach Ollama at {host}. Is it running? (`ollama serve`): {e}")
        })?;

    let status = response.status();
    let body_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read the Ollama response: {e}"))?;

    if !status.is_success() {
        let message = error_message(&body_text);
        // The overwhelmingly common cause, and the message Ollama gives is
        // not obvious about the fix.
        if message.contains("not found") {
            return Err(format!(
                "Ollama has no model called '{model}'. Pull it first: `ollama pull {model}`"
            ));
        }
        return Err(format!("Ollama error ({status}): {message}"));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("Unexpected Ollama response shape: {e}"))?;
    let raw_text = body
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| "Ollama returned no message content.".to_string())?
        .to_string();

    let cleaned = strip_code_fences(&raw_text);
    match serde_json::from_str::<ExtractedFields>(&cleaned) {
        Ok(fields) => Ok(ExtractionResult::Parsed { fields }),
        Err(err) => Ok(ExtractionResult::ParseFailed {
            raw_text,
            error: err.to_string(),
        }),
    }
}

/// Gemini's schema dialect is OpenAPI-ish rather than JSON Schema: it has
/// no type unions, so nullability is expressed with a `nullable` flag.
fn gemini_schema() -> serde_json::Value {
    let mut properties = serde_json::Map::new();
    for key in [
        "company",
        "position",
        "location",
        "work_type",
        "employment_type",
        "salary_range",
        "job_id",
        "posted_date",
        "url",
        "notes",
    ] {
        properties.insert(
            key.to_string(),
            serde_json::json!({ "type": "STRING", "nullable": true }),
        );
    }
    serde_json::json!({ "type": "OBJECT", "properties": properties })
}

/// Digs the assistant's text out of whichever response shape came back.
/// Returns None rather than erroring so the caller can report the raw body.
fn text_from_response(provider: &str, body: &serde_json::Value) -> Option<String> {
    match provider {
        "claude" => {
            let blocks = body.get("content")?.as_array()?;
            // The forced tool call puts the answer in `input` as an object.
            // Falling back to a text block keeps this working if the model
            // answers in prose anyway.
            blocks
                .iter()
                .find(|c| c.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
                .and_then(|c| c.get("input"))
                .map(|input| input.to_string())
                .or_else(|| {
                    blocks
                        .iter()
                        .find(|c| c.get("type").and_then(|t| t.as_str()) == Some("text"))
                        .and_then(|c| c.get("text"))
                        .and_then(|t| t.as_str())
                        .map(str::to_string)
                })
        }
        "openai" => body
            .get("choices")?
            .as_array()?
            .first()?
            .get("message")?
            .get("content")?
            .as_str()
            .map(str::to_string),
        "gemini" => {
            let parts = body
                .get("candidates")?
                .as_array()?
                .first()?
                .get("content")?
                .get("parts")?
                .as_array()?;
            // Gemini can split a reply across parts; join rather than take one.
            let joined: String = parts
                .iter()
                .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                .collect();
            if joined.is_empty() {
                None
            } else {
                Some(joined)
            }
        }
        _ => None,
    }
}

/// Pulls a human-readable message out of an error response. Every provider
/// nests it differently, and falling back to the raw body is better than
/// showing the user nothing.
fn error_message(body_text: &str) -> String {
    let parsed: serde_json::Value = match serde_json::from_str(body_text) {
        Ok(v) => v,
        Err(_) => return body_text.to_string(),
    };
    // Anthropic and OpenAI: { "error": { "message": ... } }
    // Gemini:               { "error": { "message": ... } } too, in practice.
    parsed
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| body_text.to_string())
}

fn provider_display_name(provider: &str) -> &str {
    match provider {
        "claude" => "Anthropic",
        "openai" => "OpenAI",
        "gemini" => "Gemini",
        other => other,
    }
}

/// Sends the screenshot to the chosen provider and parses the JSON it
/// returns. `media_type` must be one every provider accepts: image/png,
/// image/jpeg, image/webp or image/gif.
pub async fn extract_fields_from_image(
    provider: &str,
    model: &str,
    api_key: &str,
    image_base64: &str,
    media_type: &str,
) -> Result<ExtractionResult, String> {
    let name = provider_display_name(provider);
    let client = reqwest::Client::new();

    let request = match provider {
        "claude" => client
            .post(ANTHROPIC_MESSAGES_URL)
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&claude_body(model, image_base64, media_type)),
        "openai" => client
            .post(OPENAI_URL)
            .header("authorization", format!("Bearer {api_key}"))
            .json(&openai_body(model, image_base64, media_type)),
        "gemini" => client
            // The key goes in a header, not the query string, so it cannot
            // end up in a proxy log or a redirect.
            .post(format!("{GEMINI_URL_BASE}/{model}:generateContent"))
            .header("x-goog-api-key", api_key)
            .json(&gemini_body(model, image_base64, media_type)),
        other => {
            return Err(format!(
                "'{other}' has no cloud extraction. Choose Tesseract, Claude, ChatGPT or Gemini in Settings."
            ))
        }
    };

    let response = request
        .header("content-type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Failed to reach the {name} API: {e}"))?;

    let status = response.status();
    let body_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read the {name} API response: {e}"))?;

    if !status.is_success() {
        return Err(format!(
            "{name} API error ({status}): {}",
            error_message(&body_text)
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("Unexpected {name} API response shape: {e}"))?;

    let raw_text = text_from_response(provider, &body)
        .ok_or_else(|| format!("{name} returned no text content."))?;

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

    // ---- request shapes, checked without a network call or a key ----

    #[test]
    fn claude_sends_the_image_as_a_base64_source_block() {
        let b = claude_body("test-model", "BASE64DATA", "image/png");
        assert_eq!(b["model"], "test-model");
        let content = &b["messages"][0]["content"];
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[0]["source"]["type"], "base64");
        assert_eq!(content[0]["source"]["media_type"], "image/png");
        assert_eq!(content[0]["source"]["data"], "BASE64DATA");
        assert_eq!(b["system"], SYSTEM_PROMPT);
    }

    #[test]
    fn openai_sends_the_image_as_a_data_url() {
        let b = openai_body("test-model", "BASE64DATA", "image/jpeg");
        assert_eq!(b["model"], "test-model");
        assert_eq!(b["messages"][0]["role"], "system");
        assert_eq!(b["messages"][0]["content"], SYSTEM_PROMPT);
        let parts = &b["messages"][1]["content"];
        assert_eq!(parts[1]["type"], "image_url");
        assert_eq!(
            parts[1]["image_url"]["url"],
            "data:image/jpeg;base64,BASE64DATA",
            "OpenAI takes the image inline as a data URL, not as a source block"
        );
    }

    #[test]
    fn gemini_sends_the_image_as_inline_data() {
        let b = gemini_body("test-model", "BASE64DATA", "image/webp");
        assert_eq!(b["systemInstruction"]["parts"][0]["text"], SYSTEM_PROMPT);
        let parts = &b["contents"][0]["parts"];
        assert_eq!(parts[1]["inlineData"]["mimeType"], "image/webp");
        assert_eq!(parts[1]["inlineData"]["data"], "BASE64DATA");
    }

    #[test]
    fn every_provider_carries_the_same_system_prompt_and_image() {
        for body in [
            claude_body("m", "D", "image/png"),
            openai_body("m", "D", "image/png"),
            gemini_body("m", "D", "image/png"),
        ] {
            let text = body.to_string();
            assert!(text.contains("BASE64DATA") || text.contains("\"D\"") || text.contains(",D"),
                "the image must actually be in the body");
            assert!(
                text.contains("Return ONLY raw JSON"),
                "the system prompt must reach every provider"
            );
        }
    }

    // ---- response shapes ----

    #[test]
    fn reads_claudes_text_block() {
        let body = serde_json::json!({
            "content": [
                { "type": "thinking", "thinking": "hmm" },
                { "type": "text", "text": "{\"company\":\"Acme\"}" }
            ]
        });
        assert_eq!(
            text_from_response("claude", &body).unwrap(),
            "{\"company\":\"Acme\"}",
            "a non-text block before the answer must be skipped"
        );
    }

    #[test]
    fn reads_openais_message_content() {
        let body = serde_json::json!({
            "choices": [{ "message": { "role": "assistant", "content": "{\"company\":\"Acme\"}" } }]
        });
        assert_eq!(text_from_response("openai", &body).unwrap(), "{\"company\":\"Acme\"}");
    }

    #[test]
    fn joins_geminis_split_parts() {
        let body = serde_json::json!({
            "candidates": [{ "content": { "parts": [
                { "text": "{\"company\":" },
                { "text": "\"Acme\"}" }
            ]}}]
        });
        assert_eq!(
            text_from_response("gemini", &body).unwrap(),
            "{\"company\":\"Acme\"}",
            "Gemini can split a reply across parts"
        );
    }

    #[test]
    fn an_unexpected_response_is_none_rather_than_a_panic() {
        let empty = serde_json::json!({});
        for provider in ["claude", "openai", "gemini", "something-else"] {
            assert!(text_from_response(provider, &empty).is_none(), "{provider}");
        }
        assert!(text_from_response("gemini", &serde_json::json!({
            "candidates": [{ "content": { "parts": [] } }]
        }))
        .is_none());
    }

    // ---- errors ----

    #[test]
    fn pulls_the_message_out_of_an_error_body() {
        let anthropic = r#"{"type":"error","error":{"type":"invalid_request_error","message":"credit balance is too low"}}"#;
        assert_eq!(error_message(anthropic), "credit balance is too low");
        let openai = r#"{"error":{"message":"Incorrect API key provided","type":"invalid_request_error"}}"#;
        assert_eq!(error_message(openai), "Incorrect API key provided");
        let gemini = r#"{"error":{"code":400,"message":"API key not valid","status":"INVALID_ARGUMENT"}}"#;
        assert_eq!(error_message(gemini), "API key not valid");
    }

    #[test]
    fn falls_back_to_the_raw_body_when_it_is_not_json() {
        assert_eq!(error_message("502 Bad Gateway"), "502 Bad Gateway");
        assert_eq!(error_message("{}"), "{}");
    }

    #[test]
    fn provider_names_read_the_way_a_person_would_say_them() {
        assert_eq!(provider_display_name("claude"), "Anthropic");
        assert_eq!(provider_display_name("openai"), "OpenAI");
        assert_eq!(provider_display_name("gemini"), "Gemini");
    }

    // ---- enforced output shape ----

    #[test]
    fn the_schema_covers_every_field_the_app_stores() {
        let schema = field_schema();
        let props = schema["properties"].as_object().unwrap();
        for key in [
            "company", "position", "location", "work_type", "employment_type",
            "salary_range", "job_id", "posted_date", "url", "notes",
        ] {
            assert!(props.contains_key(key), "{key} missing from the schema");
        }
        assert_eq!(props.len(), 10, "no extra fields the model could invent");
        assert_eq!(schema["additionalProperties"], false);
    }

    #[test]
    fn every_field_may_be_null_so_nothing_has_to_be_invented() {
        let schema = field_schema();
        for (name, prop) in schema["properties"].as_object().unwrap() {
            let types = prop["type"].as_array().unwrap();
            assert!(
                types.iter().any(|t| t == "null"),
                "{name} must be nullable - a field that isn't visible comes back null"
            );
        }
    }

    #[test]
    fn openai_strict_mode_needs_every_property_required() {
        let schema = field_schema();
        let required: Vec<&str> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(
            required.len(),
            schema["properties"].as_object().unwrap().len(),
            "strict mode rejects a schema that omits any property from required"
        );
    }

    #[test]
    fn claude_is_forced_to_call_the_recording_tool() {
        let b = claude_body("m", "D", "image/png");
        assert_eq!(b["tools"][0]["name"], TOOL_NAME);
        assert_eq!(b["tool_choice"]["type"], "tool");
        assert_eq!(
            b["tool_choice"]["name"], TOOL_NAME,
            "without forcing it the model may answer in prose instead"
        );
        assert!(b["tools"][0]["input_schema"]["properties"]["company"].is_object());
    }

    #[test]
    fn openai_is_given_a_strict_json_schema() {
        let b = openai_body("m", "D", "image/png");
        assert_eq!(b["response_format"]["type"], "json_schema");
        assert_eq!(b["response_format"]["json_schema"]["strict"], true);
        assert!(b["response_format"]["json_schema"]["schema"]["properties"]["position"].is_object());
    }

    #[test]
    fn gemini_is_given_a_response_schema_in_its_own_dialect() {
        let b = gemini_body("m", "D", "image/png");
        assert_eq!(b["generationConfig"]["responseMimeType"], "application/json");
        let schema = &b["generationConfig"]["responseSchema"];
        assert_eq!(schema["type"], "OBJECT", "Gemini uses upper-case type names");
        assert_eq!(
            schema["properties"]["company"]["nullable"], true,
            "Gemini has no type unions - nullability is a flag"
        );
        assert_eq!(schema["properties"]["company"]["type"], "STRING");
    }

    #[test]
    fn reads_claudes_tool_call() {
        let body = serde_json::json!({
            "content": [
                { "type": "text", "text": "Let me look at that." },
                { "type": "tool_use", "name": TOOL_NAME,
                  "input": { "company": "Acme", "position": "Engineer" } }
            ]
        });
        let text = text_from_response("claude", &body).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["company"], "Acme");
        assert_eq!(
            parsed["position"], "Engineer",
            "the tool input is preferred over any preamble text"
        );
    }

    #[test]
    fn still_reads_claude_answering_in_prose() {
        // Belt and braces: if the tool call is ever absent, a text block
        // is still understood rather than failing outright.
        let body = serde_json::json!({
            "content": [{ "type": "text", "text": "{\"company\":\"Acme\"}" }]
        });
        assert_eq!(text_from_response("claude", &body).unwrap(), "{\"company\":\"Acme\"}");
    }

    #[test]
    fn ollama_is_sent_text_and_a_schema_not_an_image() {
        let b = ollama_body("qwen2.5:3b", "Amazon
Robotics Engineer");
        assert_eq!(b["model"], "qwen2.5:3b");
        assert_eq!(b["stream"], false, "the app waits for one whole answer");
        assert_eq!(b["messages"][0]["content"], SYSTEM_PROMPT);
        let user = b["messages"][1]["content"].as_str().unwrap();
        assert!(user.contains("Amazon"), "the OCR text has to reach the model");
        assert!(user.contains("Robotics Engineer"));
        assert!(
            b.get("images").is_none(),
            "a text model is given words, not pixels - Tesseract already read them"
        );
    }

    #[test]
    fn ollama_gets_the_same_schema_as_the_cloud_providers() {
        let b = ollama_body("m", "text");
        assert_eq!(b["format"], field_schema());
        assert!(b["format"]["properties"]["company"].is_object());
    }
}
