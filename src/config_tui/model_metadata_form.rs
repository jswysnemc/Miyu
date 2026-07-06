use crate::config::{format_model_tags, is_valid_model_tag, parse_context_chars, ProviderConfig};
use anyhow::{bail, Result};

/// 返回模型上下文字段值。
///
/// 参数:
/// - `provider`: Provider 配置
/// - `model`: 模型 ID
///
/// 返回:
/// - 表单展示的上下文长度
pub(super) fn context_chars_field_value(provider: &ProviderConfig, model: &str) -> String {
    provider
        .model_context_chars_for(model)
        .map(|value| value.to_string())
        .unwrap_or_default()
}

/// 返回模型标签字段值。
///
/// 参数:
/// - `provider`: Provider 配置
/// - `model`: 模型 ID
///
/// 返回:
/// - 表单展示的逗号分隔标签
pub(super) fn tags_field_value(provider: &ProviderConfig, model: &str) -> String {
    format_model_tags(provider.model_tags_for(model))
}

/// 应用模型上下文字段。
///
/// 参数:
/// - `provider`: Provider 配置
/// - `model`: 模型 ID
/// - `value`: 表单输入的上下文长度
///
/// 返回:
/// - 应用是否成功
pub(super) fn apply_context_chars_field(
    provider: &mut ProviderConfig,
    model: &str,
    value: &str,
) -> Result<()> {
    provider.set_model_context_chars_for(model, parse_context_chars(value)?);
    Ok(())
}

/// 应用模型标签字段。
///
/// 参数:
/// - `provider`: Provider 配置
/// - `model`: 模型 ID
/// - `value`: 表单输入的标签列表
///
/// 返回:
/// - 应用是否成功
pub(super) fn apply_tags_field(
    provider: &mut ProviderConfig,
    model: &str,
    value: &str,
) -> Result<()> {
    provider.set_model_tags_for(model, parse_tags_field(value)?);
    Ok(())
}

/// 解析模型标签字段。
///
/// 参数:
/// - `value`: 逗号或换行分隔的标签列表
///
/// 返回:
/// - 去重后的标签列表
fn parse_tags_field(value: &str) -> Result<Vec<String>> {
    let mut tags = Vec::new();
    for tag in value.split([',', '\n', '\r']) {
        let tag = tag.trim();
        if tag.is_empty() {
            continue;
        }
        if !is_valid_model_tag(tag) {
            bail!("invalid model tag: {tag}");
        }
        if !tags.iter().any(|item| item == tag) {
            tags.push(tag.to_string());
        }
    }
    Ok(tags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ModelMetadata;

    fn provider_with_model(model: &str) -> ProviderConfig {
        let mut provider = ProviderConfig::new_openai_compatible();
        provider.models.push(model.to_string());
        provider.default_model = model.to_string();
        provider
    }

    #[test]
    fn applies_unit_context_to_model_metadata() {
        let mut provider = provider_with_model("test-model");

        apply_context_chars_field(&mut provider, "test-model", "128k").unwrap();

        assert_eq!(
            provider
                .model_metadata
                .get("test-model")
                .and_then(|metadata| metadata.context_chars),
            Some(128_000)
        );
    }

    #[test]
    fn parses_and_deduplicates_tags() {
        let mut provider = provider_with_model("test-model");

        apply_tags_field(&mut provider, "test-model", "tool,vision,tool").unwrap();

        assert_eq!(
            provider.model_tags_for("test-model"),
            &["tool".to_string(), "vision".to_string()]
        );
    }

    #[test]
    fn rejects_unknown_tags() {
        let mut provider = provider_with_model("test-model");

        assert!(apply_tags_field(&mut provider, "test-model", "tool,unknown").is_err());
    }

    #[test]
    fn reads_legacy_context_when_metadata_is_empty() {
        let mut provider = provider_with_model("test-model");
        provider
            .model_context_chars
            .insert("test-model".to_string(), 42_000);
        provider
            .model_metadata
            .insert("other-model".to_string(), ModelMetadata::default());

        assert_eq!(context_chars_field_value(&provider, "test-model"), "42000");
    }
}
