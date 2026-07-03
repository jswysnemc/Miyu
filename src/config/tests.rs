use super::*;

#[test]
fn provider_config_can_be_saved_without_active_model() {
    let mut config = AppConfig::default();
    config.providers[0].models.clear();
    config.providers[0].default_model.clear();
    assert!(config.validate().is_ok());
}

#[test]
fn provider_model_choices_ignore_unconfigured_models() {
    let mut config = AppConfig::default();
    config.providers[0].models.clear();
    config.providers[0].default_model.clear();
    assert!(config.provider_model_choices().is_empty());
}

#[test]
fn new_openai_compatible_provider_has_no_active_model() {
    let provider = ProviderConfig::new_openai_compatible();

    assert!(provider.models.is_empty());
    assert!(provider.default_model.is_empty());
}

#[test]
fn display_readable_tool_names_defaults_enabled() {
    let display: DisplayConfig = serde_json::from_str(r#"{"tool_calls":"summary"}"#).unwrap();
    assert!(display.readable_tool_names);
}

#[test]
fn progressive_tool_loading_defaults_disabled() {
    let config = AppConfig::default();
    assert!(!config.tools.progressive_loading_enabled);
}

#[test]
fn meme_library_defaults_follow_persona() {
    let memes = MemesPluginConfig::default();
    assert_eq!(memes.library_for_persona(""), "miyu");
    assert_eq!(
        memes.library_for_persona("Custom Persona"),
        "custom-persona"
    );
    assert!(memes.auto_send_enabled);
    assert_eq!(memes.auto_send_probability, 0.2);
    assert_eq!(memes.auto_send_min_confidence, 0.8);
}
