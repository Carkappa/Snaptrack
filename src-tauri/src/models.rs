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

/// One way of turning a screenshot into fields.
///
/// The frontend renders both the method dropdown and the API-key card from
/// this list, so adding a provider here is all it takes for the UI to offer
/// it - and a provider that needs no key (Tesseract) shows no key UI at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractionProvider {
    pub id: String,
    pub label: String,
    /// Whether this provider sends anything over the network at all.
    pub needs_key: bool,
    /// Shown as the heading of the key card, e.g. "Anthropic API key".
    pub key_label: String,
    pub key_placeholder: String,
    /// Where to get one. Shown under the field.
    pub key_help: String,
    /// The model used unless the user overrides it. Empty for a provider
    /// that has no model to choose.
    pub default_model: String,
    /// Heading the method is listed under, so the choice reads as "which
    /// kind of thing" first and "which one" second.
    pub group: String,
    /// Base URL for providers that speak OpenAI's wire format. Empty for
    /// everything else, which has its own endpoint.
    pub api_base: String,
}

impl ExtractionProvider {
    fn new(
        id: &str,
        label: &str,
        needs_key: bool,
        key_label: &str,
        key_placeholder: &str,
        key_help: &str,
        default_model: &str,
        group: &str,
        api_base: &str,
    ) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            needs_key,
            key_label: key_label.to_string(),
            key_placeholder: key_placeholder.to_string(),
            key_help: key_help.to_string(),
            default_model: default_model.to_string(),
            group: group.to_string(),
            api_base: api_base.to_string(),
        }
    }
}

pub const DEFAULT_PROVIDER: &str = "tesseract";

const ON_MACHINE: &str = "On this machine - free, nothing leaves it";
const CLOUD: &str = "Cloud - needs an API key";
const UNIVERSITY: &str = "Texas A&M - free with your NetID";

/// Models known to be small, fast on a CPU, and good at returning JSON,
/// best first. Used only to choose among what the user already has pulled.
const MODEL_PREFERENCE: [&str; 6] = [
    "qwen2.5", "llama3.2", "llama3.1", "mistral", "phi3", "gemma2",
];

/// A model the app can offer, with enough detail to choose between them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelInfo {
    pub id: String,
    pub label: String,
    /// True when the model reads the screenshot itself. A text model reads
    /// what Tesseract already turned into words.
    pub vision: bool,
    pub size: String,
    /// Qualitative, from published comparisons and parameter count - not
    /// measured here. The UI says so rather than implying a benchmark.
    pub accuracy: String,
    pub hardware: String,
    pub description: String,
}

fn model(
    id: &str,
    label: &str,
    vision: bool,
    size: &str,
    accuracy: &str,
    hardware: &str,
    description: &str,
) -> OllamaModelInfo {
    OllamaModelInfo {
        id: id.to_string(),
        label: label.to_string(),
        vision,
        size: size.to_string(),
        accuracy: accuracy.to_string(),
        hardware: hardware.to_string(),
        description: description.to_string(),
    }
}

/// Models worth offering, roughly cheapest first within each kind.
///
/// Only models actually in Ollama's library: anything else cannot be
/// pulled, so listing it would be an invitation to a dead end.
pub fn ollama_catalogue() -> Vec<OllamaModelInfo> {
    vec![
        model(
            "qwen2.5:3b",
            "Qwen 2.5 3B",
            false,
            "~2 GB",
            "Good",
            "Runs on a CPU",
            "Tesseract reads the screenshot, this works out which words are the company and which are the title. The lightest thing that does the job.",
        ),
        model(
            "qwen2.5:7b",
            "Qwen 2.5 7B",
            false,
            "~4.7 GB",
            "Better",
            "Runs on a CPU, slower",
            "The same job as the 3B, with more room to get an awkward layout right.",
        ),
        model(
            "moondream",
            "Moondream 1.8B",
            true,
            "~1.7 GB",
            "Good",
            "Runs on a CPU",
            "Reads the screenshot itself, so it never sees Tesseract's mistakes. Small enough to be practical without a graphics card.",
        ),
        model(
            "granite3.2-vision:2b",
            "Granite 3.2 Vision 2B",
            true,
            "~2.4 GB",
            "Good",
            "Runs on a CPU",
            "Built for reading documents and screenshots rather than photographs.",
        ),
        model(
            "deepseek-ocr:3b",
            "DeepSeek-OCR 3B",
            true,
            "~2 GB",
            "Better",
            "Runs on a CPU",
            "Purpose-built for pulling text out of documents, which is exactly what a job posting is.",
        ),
        model(
            "qwen2.5vl:3b",
            "Qwen2.5-VL 3B",
            true,
            "~3.2 GB",
            "Better",
            "Runs on a CPU",
            "Reads the page and understands the layout, so the company and title do not depend on font-size guesswork.",
        ),
        model(
            "minicpm-v:8b",
            "MiniCPM-V 8B",
            true,
            "~5.5 GB",
            "Best",
            "Much better with a GPU",
            "One of the strongest open models at reading text in images. Worth it if you have the hardware.",
        ),
        model(
            "qwen2.5vl:7b",
            "Qwen2.5-VL 7B",
            true,
            "~6 GB",
            "Best",
            "Much better with a GPU",
            "Larger sibling of the 3B, and noticeably better on a cluttered page.",
        ),
        model(
            "llama3.2-vision:11b",
            "Llama 3.2 Vision 11B",
            true,
            "~7.9 GB",
            "Best",
            "Wants a GPU",
            "The heaviest option offered. Only sensible with a graphics card.",
        ),
    ]
}

/// Whether a model reads images. Unknown models are assumed text-only:
/// sending an image to a text model wastes a request, while sending text
/// to a vision model still works.
pub fn is_vision_model(id: &str) -> bool {
    if let Some(info) = ollama_catalogue().into_iter().find(|m| m.id == id) {
        return info.vision;
    }
    let lower = id.to_lowercase();
    // A model pulled by hand still gets recognised by the naming every
    // vision model in the library follows.
    ["vl", "vision", "llava", "minicpm-v", "moondream", "ocr"]
        .iter()
        .any(|marker| lower.contains(marker))
}

/// Model families ranked for this task, best first.
///
/// Reading a screenshot wants strong vision and a quick answer, not the
/// deepest reasoning: the page is right there, it just has to be
/// understood. Sonnet-class models sit at the sweet spot, the big
/// reasoning models are slow and dear for no gain, and the small ones are
/// a reasonable last resort.
const CLOUD_MODEL_RANK: [&str; 8] = [
    "sonnet", "gpt-4o", "gpt-4.1", "gemini-3", "opus", "gemini-2.5", "haiku", "flash",
];

/// Models that cannot read a screenshot and return fields, whatever their
/// name suggests.
fn is_unusable_for_extraction(id: &str) -> bool {
    let m = id.to_lowercase();
    ["embed", "whisper", "tts", "dall-e", "imagen", "audio", "moderation", "rerank"]
        .iter()
        .any(|bad| m.contains(bad))
}

/// Picks the best model for this task out of what a key can actually
/// reach.
///
/// The point is that nobody should have to know that a provider spells it
/// "protected.Claude Sonnet 4.6" - capitals, spaces and version included.
/// Returns None when nothing in the list is usable, so the caller can keep
/// whatever was configured.
pub fn best_cloud_model(available: &[String]) -> Option<String> {
    let usable: Vec<&String> = available
        .iter()
        .filter(|m| !is_unusable_for_extraction(m))
        .collect();
    if usable.is_empty() {
        return None;
    }

    // A dated snapshot is the same model as its alias, and the alias keeps
    // working when the snapshot is retired - so prefer the plain name.
    let undated = |m: &str| !m.chars().rev().take(8).all(|c| c.is_ascii_digit() || c == '-');

    for family in CLOUD_MODEL_RANK {
        // Prefer the highest version within a family, which sorts last:
        // "Claude Sonnet 4.6" beats "Claude Sonnet 4.1".
        let mut matches: Vec<&&String> = usable
            .iter()
            .filter(|m| m.to_lowercase().contains(family))
            .collect();
        if matches.is_empty() {
            continue;
        }
        matches.sort();
        // Prefer an undated alias within the family; fall back to the
        // newest dated snapshot when that is all there is.
        if let Some(alias) = matches.iter().rev().find(|m| undated(m)) {
            return Some((***alias).clone());
        }
        return Some((**matches.last().expect("checked non-empty")).clone());
    }
    Some(usable[0].clone())
}

#[cfg(test)]
mod cloud_model_tests {
    use super::*;

    fn list(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sonnet_class_wins_for_reading_a_screenshot() {
        // The real list a Texas A&M key returned.
        let got = best_cloud_model(&list(&[
            "protected.o3",
            "protected.Claude Opus 4.1",
            "protected.Claude Opus 4.6",
            "protected.Claude 3.5 Haiku",
            "protected.Claude-Haiku-4.5",
            "protected.Claude Sonnet 4.6",
        ]));
        assert_eq!(got.as_deref(), Some("protected.Claude Sonnet 4.6"));
    }

    #[test]
    fn the_newest_of_a_family_is_taken() {
        let got = best_cloud_model(&list(&[
            "protected.Claude Sonnet 4.1",
            "protected.Claude Sonnet 4.6",
        ]));
        assert_eq!(got.as_deref(), Some("protected.Claude Sonnet 4.6"));
    }

    #[test]
    fn a_reasoning_model_is_not_chosen_over_a_vision_one() {
        // o3 is stronger and slower, and the page is right there to read.
        let got = best_cloud_model(&list(&["o3", "gpt-4o"]));
        assert_eq!(got.as_deref(), Some("gpt-4o"));
    }

    #[test]
    fn models_that_cannot_do_this_are_skipped() {
        let got = best_cloud_model(&list(&[
            "text-embedding-3-large",
            "whisper-1",
            "dall-e-3",
            "gpt-4o",
        ]));
        assert_eq!(got.as_deref(), Some("gpt-4o"));
        assert_eq!(
            best_cloud_model(&list(&["text-embedding-3-large", "whisper-1"])),
            None,
            "nothing usable means keep whatever was configured"
        );
    }

    #[test]
    fn an_alias_is_preferred_over_a_dated_snapshot() {
        // "claude-sonnet-5" keeps working when "claude-sonnet-5-20260101"
        // is retired, so pinning to the snapshot is how a default goes
        // stale in the first place.
        let got = best_cloud_model(&list(&[
            "claude-sonnet-5-20260101",
            "claude-sonnet-5",
            "claude-opus-5",
        ]));
        assert_eq!(got.as_deref(), Some("claude-sonnet-5"));
    }

    #[test]
    fn a_dated_snapshot_is_used_when_there_is_no_alias() {
        let got = best_cloud_model(&list(&["claude-sonnet-5-20260101", "claude-sonnet-5-20260315"]));
        assert_eq!(
            got.as_deref(),
            Some("claude-sonnet-5-20260315"),
            "the newest snapshot when no alias exists"
        );
    }

    #[test]
    fn an_unranked_model_is_still_better_than_nothing() {
        let got = best_cloud_model(&list(&["some-new-model-2027"]));
        assert_eq!(got.as_deref(), Some("some-new-model-2027"));
    }

    #[test]
    fn an_empty_list_chooses_nothing() {
        assert_eq!(best_cloud_model(&[]), None);
    }
}

/// Picks a usable model out of what Ollama has actually pulled.
///
/// The point is that nothing has to be typed: a user who already has any
/// reasonable model should not be told to download another one, and a user
/// who has none should be told that rather than getting "model not found"
/// at the moment they paste a screenshot.
///
/// Embedding models are excluded - they cannot hold a conversation, so they
/// would fail in a way that looks like a bug in the app.
pub fn best_available_model(preferred: &str, installed: &[String]) -> Option<String> {
    let usable: Vec<&String> = installed
        .iter()
        .filter(|m| !m.to_lowercase().contains("embed"))
        .collect();
    if usable.is_empty() {
        return None;
    }

    let family = |m: &str| m.split(':').next().unwrap_or(m).to_lowercase();
    let want = family(preferred);

    // Exactly what was asked for, tag and all.
    if let Some(m) = usable.iter().find(|m| m.as_str() == preferred) {
        return Some((*m).clone());
    }
    // The same model at a different tag: qwen2.5:7b will do when the
    // default was qwen2.5:3b.
    if let Some(m) = usable.iter().find(|m| family(m) == want) {
        return Some((*m).clone());
    }
    // Otherwise the best of what is there.
    for pref in MODEL_PREFERENCE {
        if let Some(m) = usable.iter().find(|m| family(m).starts_with(pref)) {
            return Some((*m).clone());
        }
    }
    Some(usable[0].clone())
}

#[cfg(test)]
mod model_choice_tests {
    use super::*;

    fn installed(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_exact_model_wins_when_it_is_there() {
        let got = best_available_model("qwen2.5:3b", &installed(&["llama3.2", "qwen2.5:3b"]));
        assert_eq!(got.as_deref(), Some("qwen2.5:3b"));
    }

    #[test]
    fn a_different_tag_of_the_same_model_is_good_enough() {
        let got = best_available_model("qwen2.5:3b", &installed(&["qwen2.5:7b"]));
        assert_eq!(
            got.as_deref(),
            Some("qwen2.5:7b"),
            "no reason to make someone download a second copy of the same model"
        );
    }

    #[test]
    fn falls_back_to_the_best_of_what_is_pulled() {
        let got = best_available_model("qwen2.5:3b", &installed(&["gemma2", "llama3.2"]));
        assert_eq!(got.as_deref(), Some("llama3.2"), "ranked ahead of gemma2");
    }

    #[test]
    fn an_unranked_model_is_still_used_rather_than_nothing() {
        let got = best_available_model("qwen2.5:3b", &installed(&["some-new-model:latest"]));
        assert_eq!(got.as_deref(), Some("some-new-model:latest"));
    }

    #[test]
    fn embedding_models_are_never_chosen() {
        // These cannot chat; picking one fails in a way that looks like a
        // bug in the app rather than the wrong model.
        assert_eq!(
            best_available_model("qwen2.5:3b", &installed(&["nomic-embed-text", "mxbai-embed-large"])),
            None
        );
        assert_eq!(
            best_available_model("qwen2.5:3b", &installed(&["nomic-embed-text", "llama3.2"])).as_deref(),
            Some("llama3.2")
        );
    }

    #[test]
    fn nothing_pulled_means_nothing_chosen() {
        assert_eq!(best_available_model("qwen2.5:3b", &[]), None);
    }
}

pub fn extraction_providers() -> Vec<ExtractionProvider> {
    vec![
        ExtractionProvider::new(
            "tesseract",
            "Tesseract - text recognition only",
            false,
            "",
            "",
            "",
            "",
            ON_MACHINE,
            "",
        ),
        ExtractionProvider::new(
            "ollama",
            "Local model (Ollama) - free, offline",
            false,
            "",
            "",
            "",
            "qwen2.5:3b",
            ON_MACHINE,
            "",
        ),
        ExtractionProvider::new(
            "claude",
            "Claude (Anthropic)",
            true,
            "Anthropic API key",
            "sk-ant-...",
            "Create one at console.anthropic.com under API Keys.",
            "claude-sonnet-5",
            CLOUD,
            "",
        ),
        ExtractionProvider::new(
            "openai",
            "ChatGPT (OpenAI)",
            true,
            "OpenAI API key",
            "sk-...",
            "Create one at platform.openai.com/api-keys.",
            "gpt-4o",
            CLOUD,
            "https://api.openai.com/v1",
        ),
        ExtractionProvider::new(
            "tamu",
            "Texas A&M AI Chat",
            true,
            "TAMU AI Chat API key",
            "sk-...",
            "Sign in at chat.tamu.ai with your NetID, then create a key in Settings there.",
            "protected.gpt-4o",
            UNIVERSITY,
            "https://chat-api.tamu.ai/openai",
        ),
        ExtractionProvider::new(
            "gemini",
            "Gemini (Google)",
            true,
            "Google AI Studio API key",
            "AIza...",
            "Create one at aistudio.google.com/apikey.",
            "gemini-3.6-flash",
            CLOUD,
            "",
        ),
    ]
}

pub fn find_provider(id: &str) -> Option<ExtractionProvider> {
    extraction_providers().into_iter().find(|p| p.id == id)
}

/// Falls back to Tesseract for an id this build doesn't know - a settings
/// file written by a newer version, say. Never leaves the app without a
/// working extraction method.
pub fn provider_or_default(id: &str) -> ExtractionProvider {
    find_provider(id).unwrap_or_else(|| {
        find_provider(DEFAULT_PROVIDER).expect("the default provider must exist")
    })
}

#[cfg(test)]
mod provider_tests {
    use super::*;

    #[test]
    fn tesseract_is_the_default_and_needs_no_key() {
        let p = provider_or_default(DEFAULT_PROVIDER);
        assert_eq!(p.id, "tesseract");
        assert!(!p.needs_key);
        assert!(
            p.key_label.is_empty(),
            "an offline provider must carry no key wording for the UI to show"
        );
    }

    #[test]
    fn every_cloud_provider_describes_its_own_key() {
        for p in extraction_providers().into_iter().filter(|p| p.needs_key) {
            assert!(!p.key_label.is_empty(), "{} needs a key heading", p.id);
            assert!(!p.key_placeholder.is_empty(), "{} needs a placeholder", p.id);
            assert!(!p.key_help.is_empty(), "{} needs a hint", p.id);
            assert!(
                !p.key_label.to_lowercase().contains("anthropic") || p.id == "claude",
                "only Claude's card may mention Anthropic"
            );
            assert_ne!(p.id, "ollama", "Ollama must never be treated as needing a key");
        }
    }

    #[test]
    fn every_provider_that_runs_a_model_names_a_default() {
        for p in extraction_providers() {
            if p.needs_key {
                assert!(!p.default_model.is_empty(), "{} needs a model", p.id);
            }
        }
        assert!(
            find_provider("tesseract").unwrap().default_model.is_empty(),
            "Tesseract is an OCR engine, not a model - no model field for it"
        );
        assert!(
            !find_provider("ollama").unwrap().default_model.is_empty(),
            "Ollama needs a model even though it needs no key"
        );
    }

    #[test]
    fn ollama_is_offline_and_keyless() {
        let p = find_provider("ollama").expect("ollama must be offered");
        assert!(!p.needs_key, "a model on your own machine needs no API key");
        assert!(
            p.key_label.is_empty() && p.key_placeholder.is_empty(),
            "and must carry no key wording for the UI to show"
        );
    }

    #[test]
    fn every_provider_is_grouped_and_only_two_groups_exist() {
        let mut groups: Vec<String> = extraction_providers().into_iter().map(|p| p.group).collect();
        assert!(groups.iter().all(|g| !g.is_empty()), "an ungrouped method has nowhere to appear");
        groups.sort();
        groups.dedup();
        assert_eq!(
            groups.len(),
            3,
            "on this machine, paid cloud, and the university account are three different decisions"
        );
        assert_eq!(find_provider("ollama").unwrap().group, ON_MACHINE);
        assert_eq!(find_provider("claude").unwrap().group, CLOUD);
    }

    #[test]
    fn tamu_speaks_openais_wire_format_at_its_own_address() {
        let p = find_provider("tamu").expect("the university option must exist");
        assert_eq!(p.api_base, "https://chat-api.tamu.ai/openai");
        assert!(p.needs_key, "it still needs a key, just a free one");
        assert!(
            p.default_model.starts_with("protected."),
            "TAMU namespaces its models, and a bare model name is rejected"
        );
        assert_eq!(
            find_provider("openai").unwrap().api_base,
            "https://api.openai.com/v1",
            "the two share a wire format and differ only by address"
        );
    }

    #[test]
    fn provider_ids_are_unique() {
        let ids: Vec<String> = extraction_providers().into_iter().map(|p| p.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len());
    }

    #[test]
    fn an_unknown_provider_falls_back_rather_than_breaking() {
        assert_eq!(provider_or_default("some-future-model").id, "tesseract");
        assert_eq!(provider_or_default("").id, "tesseract");
        assert!(find_provider("some-future-model").is_none());
    }
}

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
