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

/// Public so callers that ask for JSON can clean a fenced reply.
pub fn strip_fences(text: &str) -> String {
    strip_code_fences(text)
}

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

/// OpenAI's reasoning models rejected `max_tokens` in favour of
/// `max_completion_tokens`, and a gateway in front of them inherits that.
fn uses_completion_tokens(model: &str) -> bool {
    let m = model.to_lowercase();
    // "protected.o3" and "protected.gpt-5" through a university gateway
    // are the same models with a prefix.
    ["o1", "o3", "o4", "gpt-5"]
        .iter()
        .any(|family| m.contains(family))
}

/// How much of the OpenAI API to use for a given attempt.
///
/// A gateway in front of the real thing typically forwards `model`,
/// `messages` and `stream` and rejects the rest - Texas A&M's own client
/// library sends exactly those three and nothing else. Rather than guess
/// which extras a given endpoint tolerates, the request steps down this
/// ladder until one is accepted.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Dialect {
    /// A guaranteed shape, which only the real OpenAI API supports.
    StrictSchema,
    /// Ask for JSON without a schema. Widely supported.
    JsonObject,
    /// Nothing but the three fields every gateway forwards. The prompt
    /// still asks for JSON, and a reply that isn't gets reported as
    /// unparsed rather than lost.
    Minimal,
}

/// `strict` is false when the request is a retry.
///
/// A gateway that only forwards the common parts of the API rejects a
/// strict json_schema outright, and the whole request fails rather than
/// degrading. Asking for plain JSON still works everywhere, and the prompt
/// asks for the same shape, so a retry loses the guarantee and nothing else.
fn openai_body(
    model: &str,
    image_base64: &str,
    media_type: &str,
    dialect: Dialect,
) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": model,
        "stream": false,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": [
                { "type": "text", "text": USER_PROMPT },
                { "type": "image_url", "image_url": {
                    "url": format!("data:{media_type};base64,{image_base64}") } }
            ]}
        ]
    });

    let Some(obj) = body.as_object_mut() else {
        return body;
    };
    if dialect == Dialect::Minimal {
        return body;
    }

    // Reasoning models renamed this, and a gateway in front of them
    // inherits the rename, prefix and all.
    let limit_key = if uses_completion_tokens(model) {
        "max_completion_tokens"
    } else {
        "max_tokens"
    };
    obj.insert(limit_key.to_string(), serde_json::json!(1024));
    obj.insert(
        "response_format".to_string(),
        if dialect == Dialect::StrictSchema {
            serde_json::json!({
                "type": "json_schema",
                "json_schema": { "name": TOOL_NAME, "strict": true, "schema": field_schema() }
            })
        } else {
            serde_json::json!({ "type": "json_object" })
        },
    );
    body
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
/// How long Ollama holds the model in memory after answering.
///
/// Its default is five minutes, which for this app means several gigabytes
/// sitting in RAM between captures - and captures are minutes or hours
/// apart. `0` unloads as soon as the reply is sent, at the cost of loading
/// it again next time. That matches how the rest of the app behaves: inert
/// until you invoke it.
fn keep_alive(unload_after_use: bool) -> serde_json::Value {
    if unload_after_use {
        serde_json::json!(0)
    } else {
        serde_json::json!("5m")
    }
}

fn ollama_body(model: &str, ocr_text: &str, unload_after_use: bool) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "stream": false,
        "keep_alive": keep_alive(unload_after_use),
        // Ollama enforces a JSON schema the same way the cloud providers do.
        "format": field_schema(),
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": format!("{USER_PROMPT}\n\n{ocr_text}") }
        ]
    })
}

/// Ollama's vision path: the model is handed the screenshot itself.
///
/// Worth a separate body because it skips Tesseract entirely. A vision
/// model reads the page the way a person does, so it never inherits an OCR
/// mistake and does not depend on font-size guesswork to tell a company
/// from a title.
fn ollama_vision_body(
    model: &str,
    image_base64: &str,
    unload_after_use: bool,
) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "stream": false,
        "keep_alive": keep_alive(unload_after_use),
        "format": field_schema(),
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            {
                "role": "user",
                "content": USER_PROMPT,
                // Ollama takes images on the message, base64 and unprefixed.
                "images": [image_base64]
            }
        ]
    })
}

/// Turns text already read off a screenshot into fields, using a model
/// running on this machine. No key, no network beyond localhost.
pub async fn extract_fields_from_text(
    host: &str,
    model: &str,
    ocr_text: &str,
    unload_after_use: bool,
) -> Result<ExtractionResult, String> {
    ollama_request(host, model, ollama_body(model, ocr_text, unload_after_use)).await
}

/// Sends the screenshot to a vision model, skipping Tesseract.
pub async fn extract_fields_with_ollama_vision(
    host: &str,
    model: &str,
    image_base64: &str,
    unload_after_use: bool,
) -> Result<ExtractionResult, String> {
    ollama_request(
        host,
        model,
        ollama_vision_body(model, image_base64, unload_after_use),
    )
    .await
}

async fn ollama_request(
    host: &str,
    model: &str,
    body: serde_json::Value,
) -> Result<ExtractionResult, String> {
    let url = format!("{}/api/chat", host.trim_end_matches('/'));
    let response = reqwest::Client::new()
        .post(&url)
        .header("content-type", "application/json")
        .json(&body)
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

/// Number of attempts for an error the provider says is temporary.
const TRANSIENT_ATTEMPTS: usize = 3;

/// Sends a request, retrying while the provider says it is overloaded.
///
/// A 503 means the model is busy, not that anything is wrong with the
/// request - Gemini in particular answers this way under load, and a single
/// attempt turns a momentary spike into a failed capture. Waits a little
/// longer each time rather than hammering a service that just said it is
/// struggling.
async fn send_with_retry(
    request: reqwest::RequestBuilder,
    name: &str,
) -> Result<(reqwest::StatusCode, String), String> {
    let mut wait_ms = 700;

    for attempt in 1..=TRANSIENT_ATTEMPTS {
        let attempt_request = request
            .try_clone()
            .ok_or_else(|| format!("Could not prepare the {name} request."))?;

        let response = attempt_request
            .header("content-type", "application/json")
            .send()
            .await
            .map_err(|e| format!("Failed to reach the {name} API: {e}"))?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read the {name} API response: {e}"))?;

        // 429 and 503 are the provider asking for a moment. Everything
        // else will say the same thing however many times it is asked.
        let temporary = matches!(status.as_u16(), 429 | 503);
        if !temporary || attempt == TRANSIENT_ATTEMPTS {
            return Ok((status, body_text));
        }

        tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;
        wait_ms *= 2;
    }

    unreachable!("the loop returns on the final attempt")
}

/// Sends a prompt and gets prose back, for work that is not extraction.
///
/// The rest of this module turns a screenshot into fields, which needs a
/// schema and an image. Tailoring a resume needs neither - it is text in,
/// text out - so it shares the providers and the keys but not the request
/// shape.
pub async fn chat_text(
    provider: &str,
    model: &str,
    api_key: &str,
    api_base: &str,
    system: &str,
    user: &str,
    schema: Option<serde_json::Value>,
) -> Result<String, String> {
    let name = provider_display_name(provider);
    let client = reqwest::Client::new();

    // Asked for as a schema where the provider supports one, and as a
    // plain instruction where it does not - the caller parses either way,
    // and reports a reply it cannot use rather than pasting prose into a
    // PDF.
    let json_note = if schema.is_some() {
        "\n\nReturn only JSON matching the required shape. No prose, no code fences."
    } else {
        ""
    };
    let system_text = format!("{system}{json_note}");

    let request = match provider {
        "claude" => client
            .post(ANTHROPIC_MESSAGES_URL)
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&serde_json::json!({
                "model": model,
                "max_tokens": 4096,
                "system": system_text,
                "messages": [{ "role": "user", "content": user }]
            })),
        "gemini" => {
            let mut config = serde_json::json!({ "maxOutputTokens": 4096 });
            if schema.is_some() {
                config["responseMimeType"] = serde_json::json!("application/json");
            }
            client
                .post(format!("{GEMINI_URL_BASE}/{model}:generateContent"))
                .header("x-goog-api-key", api_key)
                .json(&serde_json::json!({
                    "systemInstruction": { "parts": [{ "text": system_text }] },
                    "contents": [{ "role": "user", "parts": [{ "text": user }] }],
                    "generationConfig": config
                }))
        }
        "openai" | "tamu" => {
            let mut body = serde_json::json!({
                "model": model,
                "stream": false,
                "messages": [
                    { "role": "system", "content": system_text },
                    { "role": "user", "content": user }
                ]
            });
            // Not a strict schema: a university gateway rejects those, and
            // a rejected request here means no resume at all.
            if schema.is_some() {
                body["response_format"] = serde_json::json!({ "type": "json_object" });
            }
            client
                .post(format!("{}/chat/completions", api_base.trim_end_matches('/')))
                .header("authorization", format!("Bearer {api_key}"))
                .json(&body)
        }
        "ollama" => {
            let mut body = serde_json::json!({
                "model": model,
                "stream": false,
                "keep_alive": 0,
                "messages": [
                    { "role": "system", "content": system_text },
                    { "role": "user", "content": user }
                ]
            });
            if let Some(schema) = &schema {
                body["format"] = schema.clone();
            }
            client
                .post(format!("{}/api/chat", api_base.trim_end_matches('/')))
                .json(&body)
        }
        other => {
            return Err(format!(
                "{other} reads screenshots but cannot write. Choose a model-backed method in Settings."
            ))
        }
    };

    let (status, body_text) = send_with_retry(request, name).await?;
    if !status.is_success() {
        return Err(describe_api_error(name, status, &body_text));
    }

    let body: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("Unexpected {name} response shape: {e}"))?;
    let key = if provider == "ollama" { "ollama" } else { provider };
    let text = match key {
        "ollama" => body
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .map(str::to_string),
        _ => text_from_response(
            if provider == "openai" || provider == "tamu" { "openai" } else { provider },
            &body,
        ),
    };
    text.ok_or_else(|| format!("{name} returned nothing."))
}

/// Asks a provider which models a key can reach.
///
/// Every provider has a listing endpoint and every one shapes it
/// differently. Worth the per-provider code: without it a retired model is
/// a 404 at the moment of capture that reads like a broken key, which is
/// exactly what shipping a dead Gemini default did.
///
/// Returns an empty list on any failure - the caller then keeps whatever
/// was configured, which is the behaviour from before.
pub async fn list_models(provider: &str, api_key: &str, api_base: &str) -> Vec<String> {
    let client = reqwest::Client::new();
    let request = match provider {
        "openai" | "tamu" => client
            .get(format!("{}/models", api_base.trim_end_matches('/')))
            .header("authorization", format!("Bearer {api_key}")),
        "claude" => client
            .get("https://api.anthropic.com/v1/models?limit=100")
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION),
        "gemini" => client
            .get("https://generativelanguage.googleapis.com/v1beta/models?pageSize=200")
            .header("x-goog-api-key", api_key),
        _ => return Vec::new(),
    };

    let Ok(response) = request.send().await else {
        return Vec::new();
    };
    if !response.status().is_success() {
        return Vec::new();
    }
    let Ok(body) = response.json::<serde_json::Value>().await else {
        return Vec::new();
    };

    let mut names = match provider {
        // Gemini lists under "models", names them "models/gemini-3.6-flash",
        // and includes embedding models that cannot answer a prompt.
        "gemini" => body
            .get("models")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|m| {
                        m.get("supportedGenerationMethods")
                            .and_then(|s| s.as_array())
                            .map(|methods| methods.iter().any(|x| x.as_str() == Some("generateContent")))
                            .unwrap_or(true)
                    })
                    .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                    .map(|n| n.trim_start_matches("models/").to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        // Anthropic and the OpenAI-compatible ones both use data[].id.
        // Anthropic also says which models accept an image, which is the
        // thing that actually matters here.
        "claude" => body
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|m| {
                        m.pointer("/capabilities/image_input/supported")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true)
                    })
                    .filter_map(|m| m.get("id").and_then(|i| i.as_str()).map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        _ => body
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("id").and_then(|i| i.as_str()).map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    };
    names.sort();
    names.dedup();
    names
}

/// Turns a provider's error into something that says what to do.
///
/// The raw messages are accurate and unhelpful at the moment they appear:
/// a retired model reads as a 404, and an overloaded one as a 503 that
/// looks like the key is wrong.
fn describe_api_error(name: &str, status: reqwest::StatusCode, body: &str) -> String {
    let message = error_message(body);
    let lower = message.to_lowercase();

    let hint = match status.as_u16() {
        503 | 429 => Some(
            "the provider is busy, not you - this was retried a few times already, so try again shortly or add a fallback method",
        ),
        401 | 403 => Some("check the key, or that it is the right one for this method"),
        404 if lower.contains("model") => {
            Some("that model no longer exists - change it in Settings under Model")
        }
        _ => None,
    };

    match hint {
        Some(hint) => format!("{name} ({status}): {message} - {hint}."),
        None => format!("{name} API error ({status}): {message}"),
    }
}

fn provider_display_name(provider: &str) -> &str {
    match provider {
        "claude" => "Anthropic",
        "openai" => "OpenAI",
        "gemini" => "Gemini",
        "tamu" => "Texas A&M AI Chat",
        other => other,
    }
}

/// Talks to anything speaking OpenAI's wire format, retrying without the
/// strict schema if the far end will not take one.
///
/// A university or company gateway typically forwards the common parts of
/// the API and rejects the rest, so a strict `json_schema` fails the whole
/// request. Plain JSON mode is accepted everywhere, and the prompt already
/// asks for the same shape - the retry gives up the guarantee, not the
/// result.
#[allow(clippy::too_many_arguments)]
async fn openai_compatible(
    client: &reqwest::Client,
    api_base: &str,
    api_key: &str,
    model: &str,
    image_base64: &str,
    media_type: &str,
    name: &str,
) -> Result<ExtractionResult, String> {
    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
    let mut last_error = String::new();

    for dialect in [Dialect::StrictSchema, Dialect::JsonObject, Dialect::Minimal] {
        let request = client
            .post(&url)
            .header("authorization", format!("Bearer {api_key}"))
            .json(&openai_body(model, image_base64, media_type, dialect));
        let (status, body_text) = send_with_retry(request, name).await?;

        if status.is_success() {
            let body: serde_json::Value = serde_json::from_str(&body_text)
                .map_err(|e| format!("Unexpected {name} API response shape: {e}"))?;
            let raw_text = text_from_response("openai", &body)
                .ok_or_else(|| format!("{name} returned no text content."))?;
            let cleaned = strip_code_fences(&raw_text);
            return match serde_json::from_str::<ExtractedFields>(&cleaned) {
                Ok(fields) => Ok(ExtractionResult::Parsed { fields }),
                Err(err) => Ok(ExtractionResult::ParseFailed {
                    raw_text,
                    error: err.to_string(),
                }),
            };
        }

        last_error = describe_api_error(name, status, &body_text);

        // Only a rejected request is worth asking again more simply. A bad
        // key, a missing model or a busy service says the same thing to
        // every dialect, and trying three times would just be slower.
        if status.as_u16() != 400 || dialect == Dialect::Minimal {
            return Err(last_error);
        }
    }

    Err(last_error)
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
    api_base: &str,
) -> Result<ExtractionResult, String> {
    let name = provider_display_name(provider);
    let client = reqwest::Client::new();

    let request = match provider {
        "claude" => client
            .post(ANTHROPIC_MESSAGES_URL)
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&claude_body(model, image_base64, media_type)),
        // Texas A&M's AI Chat speaks OpenAI's wire format at its own
        // address, so the two share everything but the URL. Handled
        // separately below because it may need a second, plainer attempt.
        "openai" | "tamu" => {
            return openai_compatible(
                &client, api_base, api_key, model, image_base64, media_type, name,
            )
            .await
        }
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

    let (status, body_text) = send_with_retry(request, name).await?;

    if !status.is_success() {
        return Err(describe_api_error(name, status, &body_text));
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
        let b = openai_body("test-model", "BASE64DATA", "image/jpeg", Dialect::StrictSchema);
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
            openai_body("m", "D", "image/png", Dialect::StrictSchema),
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
        let b = openai_body("m", "D", "image/png", Dialect::StrictSchema);
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
        let b = ollama_body("qwen2.5:3b", "Amazon\nRobotics Engineer", true);
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
        let b = ollama_body("m", "text", true);
        assert_eq!(b["format"], field_schema());
        assert!(b["format"]["properties"]["company"].is_object());
    }

    #[test]
    fn the_model_is_unloaded_after_answering_by_default() {
        // Ollama holds a model for five minutes otherwise, which for this
        // app means gigabytes of RAM sitting idle between captures that are
        // hours apart.
        assert_eq!(ollama_body("m", "text", true)["keep_alive"], 0);
        assert_eq!(ollama_vision_body("m", "DATA", true)["keep_alive"], 0);
    }

    #[test]
    fn keeping_it_loaded_uses_ollamas_own_default() {
        assert_eq!(ollama_body("m", "text", false)["keep_alive"], "5m");
        assert_eq!(ollama_vision_body("m", "DATA", false)["keep_alive"], "5m");
    }

    #[test]
    fn a_vision_model_is_sent_the_image_not_the_text() {
        let b = ollama_vision_body("minicpm-v:8b", "BASE64DATA", true);
        assert_eq!(b["messages"][1]["images"][0], "BASE64DATA");
        assert_eq!(b["format"], field_schema(), "still held to the schema");
        assert_eq!(
            b["messages"][0]["content"], SYSTEM_PROMPT,
            "and told not to invent anything"
        );
    }

    #[test]
    fn reasoning_models_get_the_token_limit_they_accept() {
        // o-series and GPT-5 rejected max_tokens; a gateway in front of
        // them inherits that, prefix and all.
        for model in ["protected.o3", "o3-mini", "protected.gpt-5", "gpt-5"] {
            let b = openai_body(model, "D", "image/png", Dialect::StrictSchema);
            assert!(b.get("max_completion_tokens").is_some(), "{model}");
            assert!(b.get("max_tokens").is_none(), "{model}");
        }
        for model in ["gpt-4o", "protected.gpt-4o", "gpt-4.1"] {
            let b = openai_body(model, "D", "image/png", Dialect::StrictSchema);
            assert!(b.get("max_tokens").is_some(), "{model}");
            assert!(b.get("max_completion_tokens").is_none(), "{model}");
        }
    }

    #[test]
    fn a_plainer_retry_asks_for_json_without_a_schema() {
        let strict = openai_body("gpt-4o", "D", "image/png", Dialect::StrictSchema);
        assert_eq!(strict["response_format"]["type"], "json_schema");
        assert_eq!(strict["response_format"]["json_schema"]["strict"], true);

        let plain = openai_body("gpt-4o", "D", "image/png", Dialect::JsonObject);
        assert_eq!(
            plain["response_format"]["type"], "json_object",
            "a gateway that refuses a schema still understands JSON mode"
        );
        assert!(plain["response_format"].get("json_schema").is_none());
        // The image and the instructions are unchanged either way.
        assert_eq!(plain["messages"][0]["content"], SYSTEM_PROMPT);
        assert_eq!(plain["messages"][1]["content"][1]["type"], "image_url");
    }

    #[test]
    fn only_the_first_sentence_of_a_long_error_is_kept() {
        // Three stacked failures are unreadable if each brings a paragraph.
        let long = "Tesseract isn't installed or isn't on your PATH. Install it with `brew install tesseract` (macOS), your package manager (Linux), or from https://github.com/tesseract-ocr/tesseract (Windows), then try again.";
        let short = crate::commands::first_sentence_for_test(long);
        assert!(short.len() < 90, "got {} chars: {short}", short.len());
        assert!(short.starts_with("Tesseract isn't installed"));
    }

    #[test]
    fn a_busy_provider_is_described_as_busy_rather_than_broken() {
        let body = r#"{"error":{"message":"This model is currently experiencing high demand."}}"#;
        let msg = describe_api_error("Gemini", reqwest::StatusCode::SERVICE_UNAVAILABLE, body);
        assert!(msg.contains("busy, not you"), "got: {msg}");
        assert!(msg.contains("high demand"), "the provider's own words are kept");
    }

    #[test]
    fn a_retired_model_says_where_to_change_it() {
        let body = r#"{"error":{"message":"This model models/gemini-2.0-flash is no longer available."}}"#;
        let msg = describe_api_error("Gemini", reqwest::StatusCode::NOT_FOUND, body);
        assert!(msg.contains("Settings under Model"), "got: {msg}");
    }

    #[test]
    fn a_bad_key_points_at_the_key() {
        let body = r#"{"error":{"message":"API key not valid"}}"#;
        let msg = describe_api_error("Gemini", reqwest::StatusCode::UNAUTHORIZED, body);
        assert!(msg.contains("check the key"), "got: {msg}");
    }

    #[test]
    fn an_ordinary_error_is_passed_through_unembellished() {
        let body = r#"{"error":{"message":"Something specific went wrong"}}"#;
        let msg = describe_api_error("OpenAI", reqwest::StatusCode::INTERNAL_SERVER_ERROR, body);
        assert!(msg.contains("Something specific went wrong"));
        assert!(!msg.contains(" - "), "no invented advice: {msg}");
    }

    #[test]
    fn the_shipped_gemini_model_is_one_google_still_serves() {
        // gemini-2.0-flash was shut down, and shipping a dead default meant
        // every Gemini capture failed with a 404.
        let model = crate::models::provider_or_default("gemini").default_model;
        assert!(model.starts_with("gemini-"), "got {model}");
        assert_ne!(model, "gemini-2.0-flash", "that one is retired");
        assert_ne!(model, "gemini-1.5-flash", "so is that one");
    }

    #[test]
    fn the_minimal_dialect_sends_only_what_every_gateway_forwards() {
        // Texas A&M's own client library sends exactly model, messages and
        // stream. A gateway that forwards those and rejects the rest is why
        // this dialect exists.
        let b = openai_body("protected.gpt-4o", "D", "image/png", Dialect::Minimal);
        let obj = b.as_object().unwrap();
        let mut keys: Vec<&String> = obj.keys().collect();
        keys.sort();
        assert_eq!(keys, vec!["messages", "model", "stream"], "nothing else may be sent");
        assert_eq!(b["stream"], false);
        // The image and instructions still go, they are inside messages.
        assert_eq!(b["messages"][0]["content"], SYSTEM_PROMPT);
        assert_eq!(b["messages"][1]["content"][1]["type"], "image_url");
    }

    #[test]
    fn the_ladder_gives_up_one_thing_at_a_time() {
        let strict = openai_body("gpt-4o", "D", "image/png", Dialect::StrictSchema);
        let plain = openai_body("gpt-4o", "D", "image/png", Dialect::JsonObject);
        let minimal = openai_body("gpt-4o", "D", "image/png", Dialect::Minimal);

        assert_eq!(strict["response_format"]["type"], "json_schema");
        assert_eq!(plain["response_format"]["type"], "json_object");
        assert!(minimal.get("response_format").is_none());

        assert!(strict.get("max_tokens").is_some());
        assert!(plain.get("max_tokens").is_some());
        assert!(
            minimal.get("max_tokens").is_none(),
            "the token limit goes with the last step, since a gateway that              refuses response_format often refuses this too"
        );
    }

    #[test]
    fn a_message_with_a_curly_quote_is_still_cut_correctly() {
        // find() gives a byte offset; taking that many characters keeps too
        // much the moment the text is not pure ASCII, and providers do use
        // curly quotes.
        let msg = "Couldn\u{2019}t reach the service. Install it with `brew install tesseract` and try again.";
        let short = crate::commands::first_sentence_for_test(msg);
        assert_eq!(short, "Couldn\u{2019}t reach the service...");
        assert!(!short.contains("brew"), "the advice is dropped: {short}");
    }

    #[test]
    fn a_long_first_sentence_is_cut_at_a_word() {
        let msg = "The request failed because ".to_string() + &"something ".repeat(30);
        let short = crate::commands::first_sentence_for_test(&msg);
        assert!(short.chars().count() < 180, "still trimmed: {short}");
        assert!(short.ends_with("..."));
        assert!(
            !short.trim_end_matches("...").ends_with("someth"),
            "cut at a space, not mid-word: {short}"
        );
    }

    #[test]
    fn a_short_message_is_left_exactly_as_it_is() {
        let msg = "API key not valid";
        assert_eq!(crate::commands::first_sentence_for_test(msg), msg);
    }
}
