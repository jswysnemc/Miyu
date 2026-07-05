use super::app_state::WebAppState;
use super::error::ApiResult;
use crate::config::AppConfig;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

const SECRET_MASK: &str = "********";

#[derive(Debug, Deserialize)]
pub(crate) struct SaveConfigRequest {
    config: AppConfig,
}

/// 返回服务健康状态。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 健康状态 JSON
pub(crate) async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "service": "miyu-web",
    }))
}

/// 读取脱敏配置。
///
/// 参数:
/// - `state`: Web 应用共享状态
///
/// 返回:
/// - 脱敏配置 JSON
pub(crate) async fn get_config(
    State(state): State<WebAppState>,
) -> ApiResult<Json<serde_json::Value>> {
    AppConfig::init_files(&state.paths)?;
    let config = AppConfig::load_or_default(&state.paths)?;
    Ok(Json(json!({
        "ok": true,
        "config": redact_config(config),
    })))
}

/// 保存配置。
///
/// 参数:
/// - `state`: Web 应用共享状态
/// - `payload`: 配置保存请求
///
/// 返回:
/// - 保存结果 JSON
pub(crate) async fn save_config(
    State(state): State<WebAppState>,
    Json(payload): Json<SaveConfigRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    AppConfig::init_files(&state.paths)?;
    let current = AppConfig::load_or_default(&state.paths)?;
    let mut next = payload.config;
    preserve_masked_secrets(&mut next, &current);
    next.validate()?;
    next.save(&state.paths)?;
    Ok(Json(json!({
        "ok": true,
        "config": redact_config(next),
    })))
}

/// 返回脱敏后的配置。
///
/// 参数:
/// - `config`: 原始配置
///
/// 返回:
/// - 脱敏配置
fn redact_config(mut config: AppConfig) -> AppConfig {
    for provider in &mut config.providers {
        if provider
            .api_key
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        {
            provider.api_key = Some(SECRET_MASK.to_string());
        }
    }
    if !config.gateways.qq.token.is_empty() {
        config.gateways.qq.token = SECRET_MASK.to_string();
    }
    if !config.gateways.qq.client_secret.is_empty() {
        config.gateways.qq.client_secret = SECRET_MASK.to_string();
    }
    if !config.gateways.weixin.token.is_empty() {
        config.gateways.weixin.token = SECRET_MASK.to_string();
    }
    redact_vec(&mut config.plugins.web.tinyfish_api_keys);
    redact_vec(&mut config.plugins.web.tavily_api_keys);
    redact_vec(&mut config.plugins.web.firecrawl_api_keys);
    redact_vec(&mut config.plugins.web.anysearch_api_keys);
    if !config.plugins.exchange_rate.api_key.is_empty() {
        config.plugins.exchange_rate.api_key = SECRET_MASK.to_string();
    }
    redact_vec(&mut config.plugins.image_generation.api_keys);
    config
}

/// 保留仍为掩码的密钥字段。
///
/// 参数:
/// - `next`: 待保存配置
/// - `current`: 当前配置
///
/// 返回:
/// - 无
fn preserve_masked_secrets(next: &mut AppConfig, current: &AppConfig) {
    for provider in &mut next.providers {
        if provider.api_key.as_deref() == Some(SECRET_MASK) {
            provider.api_key = current
                .providers
                .iter()
                .find(|item| item.id == provider.id)
                .and_then(|item| item.api_key.clone());
        }
    }
    preserve_masked_string(&mut next.gateways.qq.token, &current.gateways.qq.token);
    preserve_masked_string(
        &mut next.gateways.qq.client_secret,
        &current.gateways.qq.client_secret,
    );
    preserve_masked_string(
        &mut next.gateways.weixin.token,
        &current.gateways.weixin.token,
    );
    preserve_masked_vec(
        &mut next.plugins.web.tinyfish_api_keys,
        &current.plugins.web.tinyfish_api_keys,
    );
    preserve_masked_vec(
        &mut next.plugins.web.tavily_api_keys,
        &current.plugins.web.tavily_api_keys,
    );
    preserve_masked_vec(
        &mut next.plugins.web.firecrawl_api_keys,
        &current.plugins.web.firecrawl_api_keys,
    );
    preserve_masked_vec(
        &mut next.plugins.web.anysearch_api_keys,
        &current.plugins.web.anysearch_api_keys,
    );
    preserve_masked_string(
        &mut next.plugins.exchange_rate.api_key,
        &current.plugins.exchange_rate.api_key,
    );
    preserve_masked_vec(
        &mut next.plugins.image_generation.api_keys,
        &current.plugins.image_generation.api_keys,
    );
}

/// 将密钥列表脱敏。
///
/// 参数:
/// - `values`: 密钥列表
///
/// 返回:
/// - 无
fn redact_vec(values: &mut Vec<String>) {
    for value in values {
        if !value.trim().is_empty() {
            *value = SECRET_MASK.to_string();
        }
    }
}

/// 保留掩码字符串。
///
/// 参数:
/// - `next`: 待保存字段
/// - `current`: 当前字段
///
/// 返回:
/// - 无
fn preserve_masked_string(next: &mut String, current: &str) {
    if next == SECRET_MASK {
        *next = current.to_string();
    }
}

/// 保留掩码列表。
///
/// 参数:
/// - `next`: 待保存密钥列表
/// - `current`: 当前密钥列表
///
/// 返回:
/// - 无
fn preserve_masked_vec(next: &mut Vec<String>, current: &[String]) {
    if next.iter().all(|value| value == SECRET_MASK) && next.len() == current.len() {
        *next = current.to_vec();
    }
}
