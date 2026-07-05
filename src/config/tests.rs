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
fn display_wait_detail_options_default_enabled() {
    let display: DisplayConfig = serde_json::from_str(r#"{"tool_calls":"summary"}"#).unwrap();
    assert!(display.wait_show_model);
    assert!(display.wait_show_thinking_level);
}

#[test]
fn display_wait_detail_options_can_be_disabled() {
    let display: DisplayConfig =
        serde_json::from_str(r#"{"wait_show_model":false,"wait_show_thinking_level":false}"#)
            .unwrap();
    assert!(!display.wait_show_model);
    assert!(!display.wait_show_thinking_level);
}

#[test]
fn progressive_tool_loading_defaults_disabled() {
    let config = AppConfig::default();
    assert!(!config.tools.progressive_loading_enabled);
}

#[test]
fn background_command_defaults_are_enabled() {
    let config = AppConfig::default();
    assert!(config.tools.background_commands_enabled);
    assert_eq!(config.tools.background_command_timeout_seconds, 0);
    assert!(config.tools.background_command_log_max_bytes > 0);
    assert!(config.tools.background_command_stop_grace_seconds > 0);
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
