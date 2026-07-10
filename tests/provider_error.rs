use ai_human::provider_error::provider_error_hint;

#[test]
fn missing_openai_api_key_gets_actionable_hint() {
    let hint = provider_error_hint("environment variable not found").unwrap();

    assert!(hint.contains("OPENAI_API_KEY"));
    assert!(hint.contains("Next: set OPENAI_API_KEY"));
    assert!(hint.contains("ai-human doctor"));
}

#[test]
fn unsupported_provider_gets_actionable_hint() {
    let hint = provider_error_hint("unsupported model provider: ollama").unwrap();

    assert!(hint.contains("unsupported model provider"));
    assert!(hint.contains("AI_HUMAN_MODEL_PROVIDER=openai"));
    assert!(hint.contains("Next:"));
}

#[test]
fn invalid_wire_api_gets_actionable_hint() {
    let hint = provider_error_hint(
        "unsupported AI_HUMAN_OPENAI_WIRE_API value: legacy; expected responses or chat-completions",
    )
    .unwrap();

    assert!(hint.contains("AI_HUMAN_OPENAI_WIRE_API"));
    assert!(hint.contains("responses"));
    assert!(hint.contains("chat-completions"));
    assert!(hint.contains("Next:"));
}

#[test]
fn endpoint_status_error_gets_base_url_hint() {
    let hint = provider_error_hint("InvalidStatusCode: 404 Not Found").unwrap();

    assert!(hint.contains("OPENAI_BASE_URL"));
    assert!(hint.contains("AI_HUMAN_OPENAI_WIRE_API"));
    assert!(hint.contains("Next:"));
}

#[test]
fn unrelated_error_has_no_provider_hint() {
    assert!(provider_error_hint("path must reference an existing project file").is_none());
}
