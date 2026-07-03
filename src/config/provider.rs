use super::defaults::*;
use super::model::*;
use crate::default_models::{
    OPENCODE_DEFAULT_CHAT_MODEL, OPENCODE_PROVIDER_ID, OPENCODE_ZEN_BASE_URL,
};
use crate::paths::MiyuPaths;
use anyhow::{bail, Context, Result};
use std::collections::HashMap;

impl ProviderConfig {
    pub fn default_opencodezen() -> Self {
        Self {
            id: OPENCODE_PROVIDER_ID.to_string(),
            display_name: "opencode Zen".to_string(),
            base_url: OPENCODE_ZEN_BASE_URL.to_string(),
            protocol: default_provider_protocol(),
            api_key: None,
            models: vec![OPENCODE_DEFAULT_CHAT_MODEL.to_string()],
            model_context_chars: HashMap::new(),
            default_model: OPENCODE_DEFAULT_CHAT_MODEL.to_string(),
            timeout_seconds: default_timeout(),
            temperature: default_temperature(),
            thinking_level: default_thinking_level(),
            thinking_format: default_thinking_format(),
            extra_body: String::new(),
        }
    }

    pub fn default_openai() -> Self {
        Self {
            id: "openai".to_string(),
            display_name: "OpenAI-compatible".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            protocol: default_provider_protocol(),
            api_key: Some("$env:OPENAI_API_KEY".to_string()),
            models: vec!["gpt-4o-mini".to_string()],
            model_context_chars: HashMap::new(),
            default_model: "gpt-4o-mini".to_string(),
            timeout_seconds: default_timeout(),
            temperature: default_temperature(),
            thinking_level: default_thinking_level(),
            thinking_format: default_thinking_format(),
            extra_body: String::new(),
        }
    }

    pub fn default_templates() -> Vec<Self> {
        let mut providers = vec![Self::default_opencodezen()];
        providers.extend([
            Self::template("openai", "OpenAI", "https://api.openai.com/v1"),
            Self::template("deepseek", "DeepSeek", "https://api.deepseek.com"),
            Self::template(
                "gemini",
                "Gemini",
                "https://generativelanguage.googleapis.com/v1beta/openai",
            ),
            Self::template(
                "xiaomi",
                "Xiaomi",
                "https://token-plan-sgp.xiaomimimo.com/v1",
            ),
            Self::template("minimax", "Minimax", "https://api.minimaxi.com/v1"),
            Self::template("openrouter", "OpenRouter", "https://openrouter.ai/api/v1"),
            Self::template("ollama", "Ollama", "http://localhost:11434/v1"),
            Self::template("lmstudio", "LMStudio", "http://localhost:1234/v1"),
        ]);
        providers
    }

    fn template(id: &str, display_name: &str, base_url: &str) -> Self {
        Self {
            id: id.to_string(),
            display_name: display_name.to_string(),
            base_url: base_url.to_string(),
            protocol: default_provider_protocol(),
            api_key: None,
            models: Vec::new(),
            model_context_chars: HashMap::new(),
            default_model: String::new(),
            timeout_seconds: default_timeout(),
            temperature: default_temperature(),
            thinking_level: default_thinking_level(),
            thinking_format: default_thinking_format(),
            extra_body: String::new(),
        }
    }

    pub fn new_openai_compatible() -> Self {
        let mut provider = Self::default_openai();
        provider.models.clear();
        provider.default_model.clear();
        provider
    }

    pub fn resolved_api_key(&self, paths: &MiyuPaths) -> Result<String> {
        if let Some(api_key) = self.api_key.as_deref() {
            if let Some(env_name) = api_key.strip_prefix("$env:") {
                return std::env::var(env_name)
                    .with_context(|| format!("environment variable {env_name} is not set"));
            }
            if !api_key.is_empty() {
                return Ok(api_key.to_string());
            }
        }

        let secrets = SecretsConfig::load(paths)?;
        if let Some(api_key) = secrets
            .api_keys
            .get(&self.id)
            .cloned()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(api_key);
        }

        if self.is_opencode_zen() {
            return Ok("public".to_string());
        }

        bail!("missing API key for provider {}", self.id)
    }

    pub fn is_opencode_zen(&self) -> bool {
        matches!(self.id.as_str(), OPENCODE_PROVIDER_ID | "opencodezen")
            && self.base_url.trim_end_matches('/') == OPENCODE_ZEN_BASE_URL
    }
}
