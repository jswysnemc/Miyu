use super::app_state::WebAppState;
use super::error::{ApiError, ApiResult};
use crate::config::AppConfig;
use crate::gateways::manager::{
    gateway_runtime_statuses, start_gateway as start_managed_gateway,
    stop_gateway as stop_managed_gateway, ManagedGateway,
};
use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;

/// 列出网关运行状态。
///
/// 参数:
/// - `state`: Web 应用共享状态
///
/// 返回:
/// - 网关状态 JSON
pub(crate) async fn list_gateways(
    State(state): State<WebAppState>,
) -> ApiResult<Json<serde_json::Value>> {
    AppConfig::init_files(&state.paths)?;
    let config = AppConfig::load_or_default(&state.paths)?;
    let statuses = gateway_runtime_statuses(&state.paths, &config).await?;
    let gateways = statuses
        .into_iter()
        .map(|status| {
            json!({
                "id": status.gateway.id(),
                "title": status.gateway.title(),
                "enabled": status.enabled,
                "task_id": status.task_id,
                "status": status.status,
                "pid": status.pid,
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "ok": true,
        "gateways": gateways,
    })))
}

/// 启动指定网关。
///
/// 参数:
/// - `state`: Web 应用共享状态
/// - `gateway`: 网关 ID
///
/// 返回:
/// - 启动结果 JSON
pub(crate) async fn start_gateway(
    State(state): State<WebAppState>,
    Path(gateway): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    AppConfig::init_files(&state.paths)?;
    let config = AppConfig::load_or_default(&state.paths)?;
    let gateway = parse_gateway(&gateway)?;
    let raw = start_managed_gateway(&state.paths, &config, gateway).await?;
    Ok(Json(
        serde_json::from_str(&raw).unwrap_or_else(|_| json!({"ok": true})),
    ))
}

/// 停止指定网关。
///
/// 参数:
/// - `state`: Web 应用共享状态
/// - `gateway`: 网关 ID
///
/// 返回:
/// - 停止结果 JSON
pub(crate) async fn stop_gateway(
    State(state): State<WebAppState>,
    Path(gateway): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    AppConfig::init_files(&state.paths)?;
    let config = AppConfig::load_or_default(&state.paths)?;
    let gateway = parse_gateway(&gateway)?;
    let stopped = stop_managed_gateway(&state.paths, &config, gateway).await?;
    Ok(Json(json!({
        "ok": true,
        "gateway": gateway.id(),
        "stopped": stopped,
    })))
}

/// 解析网关 ID。
///
/// 参数:
/// - `value`: 网关 ID
///
/// 返回:
/// - 受管理网关
fn parse_gateway(value: &str) -> ApiResult<ManagedGateway> {
    match value {
        "qq" => Ok(ManagedGateway::Qq),
        "weixin" => Ok(ManagedGateway::Weixin),
        _ => Err(ApiError::bad_request(format!(
            "unsupported gateway: {value}"
        ))),
    }
}
