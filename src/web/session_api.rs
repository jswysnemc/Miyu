use super::app_state::WebAppState;
use super::error::{ApiError, ApiResult};
use super::session_store;
use crate::state::StateStore;
use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;

/// 返回 Web 会话列表。
///
/// 参数:
/// - `state`: Web 应用共享状态
///
/// 返回:
/// - 会话列表 JSON
pub(crate) async fn list_sessions(
    State(state): State<WebAppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let sessions = session_store::list_sessions(&state.paths)?;
    Ok(Json(json!({
        "ok": true,
        "sessions": sessions,
    })))
}

/// 创建 Web 会话。
///
/// 参数:
/// - `state`: Web 应用共享状态
///
/// 返回:
/// - 新会话 JSON
pub(crate) async fn create_session(
    State(state): State<WebAppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let session = session_store::create_session(&state.paths)?;
    Ok(Json(json!({
        "ok": true,
        "session": session,
    })))
}

/// 读取 Web 会话历史。
///
/// 参数:
/// - `state`: Web 应用共享状态
/// - `session_id`: 会话 ID
///
/// 返回:
/// - 会话历史 JSON
pub(crate) async fn get_session_history(
    State(state): State<WebAppState>,
    Path(session_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    session_store::ensure_session(&state.paths, &session_id)?;
    let scoped = session_store::session_paths(&state.paths, &session_id);
    let store = StateStore::new(&scoped).map_err(ApiError::internal)?;
    store.init_files().map_err(ApiError::internal)?;
    let entries = store.load_conversation().map_err(ApiError::internal)?;
    Ok(Json(json!({
        "ok": true,
        "messages": entries,
    })))
}

/// 删除 Web 会话。
///
/// 参数:
/// - `state`: Web 应用共享状态
/// - `session_id`: 会话 ID
///
/// 返回:
/// - 删除结果 JSON
pub(crate) async fn delete_session(
    State(state): State<WebAppState>,
    Path(session_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let deleted = session_store::delete_session(&state.paths, &session_id)?;
    Ok(Json(json!({
        "ok": true,
        "deleted": deleted,
    })))
}
