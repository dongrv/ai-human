pub fn provider_error_hint(error: &str) -> Option<String> {
    let normalized = error.to_ascii_lowercase();

    if normalized.contains("environment variable not found") {
        return Some(
            "OPENAI_API_KEY is not configured. Next: set OPENAI_API_KEY in your shell or project .env, then run `ai-human doctor`."
                .into(),
        );
    }

    if normalized.contains("unsupported model provider") {
        return Some(format!(
            "{error}. Next: set AI_HUMAN_MODEL_PROVIDER=openai or remove the unsupported provider override, then run `ai-human doctor`."
        ));
    }

    if normalized.contains("ai_human_openai_wire_api") {
        return Some(format!(
            "{error}. Next: set AI_HUMAN_OPENAI_WIRE_API=responses for /responses proxies or chat-completions for /chat/completions proxies."
        ));
    }

    if normalized.contains("invalidstatuscode")
        || normalized.contains("invalid status")
        || normalized.contains("status code")
    {
        return Some(format!(
            "{error}. Next: verify OPENAI_BASE_URL points at the API root and AI_HUMAN_OPENAI_WIRE_API matches the proxy endpoint."
        ));
    }

    None
}
