use codex_models_manager::bundled_models_response;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelVisibility;
use codex_protocol::openai_models::ModelsResponse;

const FALLBACK_METADATA_MODEL: &str = "gpt-5.4";

// Chat-capable model IDs returned by TAMU AI Chat's `/models` endpoint on 2026-07-15.
// Keep the preferred default first. Embedding-only models intentionally do not appear here.
const TAMU_AI_CHAT_MODELS: &[&str] = &[
    "gpt-5.4",
    "gpt-5.4-mini",
    "gpt-5.4-nano",
    "gpt-5.5",
    "gpt-5.2",
    "gpt-5.1",
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-nano",
    "Claude Sonnet 4.6",
    "Claude Sonnet 4.5",
    "Claude Sonnet 4",
    "Claude Opus 4.8",
    "Claude Opus 4.7",
    "Claude Opus 4.6",
    "Claude Opus 4.5",
    "Claude Opus 4.1",
    "Claude-Haiku-4.5",
    "Claude 3.5 Haiku",
    "gemini-3.5-flash",
    "gemini-3.1-flash-lite",
    "gemini-2.5-pro",
    "gemini-2.5-flash",
    "gemini-2.5-flash-lite",
    "o3",
    "o3-mini",
    "o4-mini",
    "gpt-4o",
    "gpt-4.1",
    "gpt-4.1-mini",
    "gpt-4.1-nano",
    "llama3.2",
];

pub(crate) fn static_model_catalog() -> ModelsResponse {
    let bundled = bundled_models_response()
        .unwrap_or_else(|err| panic!("bundled models.json should parse: {err}"));
    let fallback = bundled
        .models
        .iter()
        .find(|model| model.slug == FALLBACK_METADATA_MODEL)
        .unwrap_or_else(|| panic!("bundled models.json should include {FALLBACK_METADATA_MODEL}"));

    ModelsResponse {
        models: TAMU_AI_CHAT_MODELS
            .iter()
            .enumerate()
            .map(|(priority, model_id)| {
                let bundled_model = bundled.models.iter().find(|model| model.slug == *model_id);
                tamu_model(
                    bundled_model.unwrap_or(fallback),
                    model_id,
                    priority as i32,
                    bundled_model.is_some(),
                )
            })
            .collect(),
    }
}

fn tamu_model(
    metadata: &ModelInfo,
    model_id: &str,
    priority: i32,
    has_native_metadata: bool,
) -> ModelInfo {
    let mut model = metadata.clone();
    model.slug = model_id.to_string();
    model.priority = priority;
    model.visibility = ModelVisibility::List;
    model.supported_in_api = true;
    model.availability_nux = None;
    model.upgrade = None;
    model.additional_speed_tiers.clear();
    model.service_tiers.clear();
    model.default_service_tier = None;
    model.used_fallback_model_metadata = false;

    if !has_native_metadata {
        model.display_name = model_id.to_string();
        model.description = Some(format!("{model_id} via TAMU AI Chat"));
        model.default_reasoning_level = None;
        model.supported_reasoning_levels.clear();
        model.support_verbosity = false;
        model.default_verbosity = None;
    }

    model
}
