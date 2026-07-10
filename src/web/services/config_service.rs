use crate::config::AppConfig;
use crate::paths::MiyuPaths;
use anyhow::{Context, Result};
use serde_json::Value;

pub(crate) const SECRET_SENTINEL: &str = "__MIYU_SECRET_UNCHANGED__";

/// 读取并脱敏 Miyu 配置。
///
/// 参数:
/// - `paths`: Miyu 路径集合
///
/// 返回:
/// - 脱敏后的 JSON 配置
pub(crate) fn load_redacted(paths: &MiyuPaths) -> Result<Value> {
    let config = AppConfig::load_or_default(paths)?;
    let mut value = serde_json::to_value(config)?;
    redact_value(&mut value, None);
    Ok(value)
}

/// 合并敏感字段保留标记并保存配置。
///
/// 参数:
/// - `paths`: Miyu 路径集合
/// - `submitted`: 浏览器提交配置
///
/// 返回:
/// - 保存后的脱敏配置
pub(crate) fn save(paths: &MiyuPaths, mut submitted: Value) -> Result<Value> {
    let current = serde_json::to_value(AppConfig::load_or_default(paths)?)?;
    merge_secret_sentinels(&mut submitted, &current);
    let config: AppConfig =
        serde_json::from_value(submitted).context("invalid Miyu configuration")?;
    config.validate()?;
    config.save(paths)?;
    load_redacted(paths)
}

/// 递归隐藏配置中的敏感字符串。
fn redact_value(value: &mut Value, key: Option<&str>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                redact_value(value, Some(key));
            }
        }
        Value::Array(values) => {
            for value in values {
                redact_value(value, key);
            }
        }
        Value::String(text) if key.is_some_and(is_sensitive_key) => {
            if !text.trim().is_empty() && !text.trim_start().starts_with("$env:") {
                *text = SECRET_SENTINEL.to_string();
            }
        }
        _ => {}
    }
}

/// 使用当前配置替换浏览器传回的敏感字段保留标记。
fn merge_secret_sentinels(submitted: &mut Value, current: &Value) {
    match (submitted, current) {
        (Value::Object(submitted), Value::Object(current)) => {
            for (key, value) in submitted {
                if let Some(current_value) = current.get(key) {
                    merge_secret_sentinels(value, current_value);
                }
            }
        }
        (Value::Array(submitted), Value::Array(current)) => {
            for (index, value) in submitted.iter_mut().enumerate() {
                if let Some(current_value) = current.get(index) {
                    merge_secret_sentinels(value, current_value);
                }
            }
        }
        (Value::String(value), current) if value == SECRET_SENTINEL => {
            *value = current.as_str().unwrap_or_default().to_string();
        }
        _ => {}
    }
}

/// 判断配置键是否包含敏感凭据。
fn is_sensitive_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "api_key"
            | "token"
            | "client_secret"
            | "access_token"
            | "authorization"
            | "password"
            | "webhook_url"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_and_restores_sensitive_values() {
        let current = json!({
            "providers": [{ "api_key": "secret", "base_url": "https://example.test" }],
            "gateways": { "qq": { "token": "gateway-secret" } }
        });
        let mut redacted = current.clone();
        redact_value(&mut redacted, None);
        assert_eq!(redacted["providers"][0]["api_key"], SECRET_SENTINEL);
        assert_eq!(redacted["gateways"]["qq"]["token"], SECRET_SENTINEL);
        merge_secret_sentinels(&mut redacted, &current);
        assert_eq!(redacted, current);
    }

    #[test]
    fn keeps_environment_references_visible() {
        let mut value = json!({ "api_key": "$env:OPENAI_API_KEY" });
        redact_value(&mut value, None);
        assert_eq!(value["api_key"], "$env:OPENAI_API_KEY");
    }
}
