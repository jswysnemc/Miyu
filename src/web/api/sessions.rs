use super::super::app_state::WebAppState;
use super::super::error::{WebError, WebResult};
use crate::state::StateStore;
use axum::extract::{Path, Query, State};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SessionResponse {
    id: String,
    title: String,
    created_at: String,
    updated_at: String,
    active: bool,
}

#[derive(Deserialize)]
struct CreateSessionRequest {
    title: Option<String>,
}

#[derive(Deserialize)]
struct RenameSessionRequest {
    title: String,
}

#[derive(Deserialize)]
struct BulkDeleteSessionsRequest {
    ids: Vec<String>,
}

#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<usize>,
}

#[derive(Serialize)]
struct DeleteResponse {
    deleted: bool,
}

#[derive(Serialize)]
struct BulkDeleteResponse {
    deleted_ids: Vec<String>,
}

/// 返回会话管理路由。
pub(super) fn routes() -> Router<WebAppState> {
    Router::new()
        .route("/api/sessions", get(list).post(create))
        .route("/api/sessions/bulk-delete", post(remove_many))
        .route("/api/sessions/:id", patch(rename).delete(remove))
        .route("/api/sessions/:id/switch", post(switch))
        .route("/api/sessions/:id/messages", get(messages))
        .route("/api/sessions/:id/timeline", get(timeline))
}

/// 列出当前工作区会话。
async fn list(State(state): State<WebAppState>) -> WebResult<Json<Vec<SessionResponse>>> {
    let active = crate::state::active_session(&state.paths).map_err(WebError::from)?;
    let sessions = crate::state::list_sessions(&state.paths).map_err(WebError::from)?;
    Ok(Json(
        sessions
            .into_iter()
            .map(|session| SessionResponse {
                active: session.id == active.id,
                id: session.id,
                title: session.title,
                created_at: session.created_at,
                updated_at: session.updated_at,
            })
            .collect(),
    ))
}

/// 创建并切换到新会话。
async fn create(
    State(state): State<WebAppState>,
    Json(request): Json<CreateSessionRequest>,
) -> WebResult<Json<SessionResponse>> {
    reject_during_run(&state).await?;
    let session = crate::state::create_session(&state.paths, request.title.as_deref())
        .map_err(WebError::from)?;
    Ok(Json(SessionResponse {
        id: session.id,
        title: session.title,
        created_at: session.created_at,
        updated_at: session.updated_at,
        active: true,
    }))
}

/// 切换当前会话。
async fn switch(
    State(state): State<WebAppState>,
    Path(id): Path<String>,
) -> WebResult<Json<SessionResponse>> {
    reject_during_run(&state).await?;
    let session = crate::state::switch_session(&state.paths, &id)
        .map_err(|error| WebError::not_found(error.to_string()))?;
    Ok(Json(SessionResponse {
        id: session.id,
        title: session.title,
        created_at: session.created_at,
        updated_at: session.updated_at,
        active: true,
    }))
}

/// 重命名会话。
async fn rename(
    State(state): State<WebAppState>,
    Path(id): Path<String>,
    Json(request): Json<RenameSessionRequest>,
) -> WebResult<Json<SessionResponse>> {
    let session = crate::state::rename_session(&state.paths, &id, &request.title)
        .map_err(|error| WebError::bad_request(error.to_string()))?;
    let active = crate::state::active_session(&state.paths).map_err(WebError::from)?;
    Ok(Json(SessionResponse {
        active: session.id == active.id,
        id: session.id,
        title: session.title,
        created_at: session.created_at,
        updated_at: session.updated_at,
    }))
}

/// 删除会话。
async fn remove(
    State(state): State<WebAppState>,
    Path(id): Path<String>,
) -> WebResult<Json<DeleteResponse>> {
    reject_during_run(&state).await?;
    let deleted = crate::state::delete_session(&state.paths, &id)
        .map_err(|error| WebError::bad_request(error.to_string()))?;
    Ok(Json(DeleteResponse { deleted }))
}

/// 批量删除会话。
///
/// 参数:
/// - `state`: Web 应用状态
/// - `request`: 待删除会话 ID 列表
///
/// 返回:
/// - 实际删除的会话 ID 列表
async fn remove_many(
    State(state): State<WebAppState>,
    Json(request): Json<BulkDeleteSessionsRequest>,
) -> WebResult<Json<BulkDeleteResponse>> {
    reject_during_run(&state).await?;
    if request.ids.is_empty() {
        return Err(WebError::bad_request("session ids cannot be empty"));
    }
    let deleted_ids = crate::state::delete_sessions(&state.paths, &request.ids)
        .map_err(|error| WebError::bad_request(error.to_string()))?;
    Ok(Json(BulkDeleteResponse { deleted_ids }))
}

/// 读取指定会话消息历史。
async fn messages(
    State(state): State<WebAppState>,
    Path(id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> WebResult<Json<Vec<crate::state::StoredConversationEntry>>> {
    reject_during_run(&state).await?;
    crate::state::switch_session(&state.paths, &id)
        .map_err(|error| WebError::not_found(error.to_string()))?;
    let store = StateStore::new(&state.paths).map_err(WebError::from)?;
    let history = store
        .history(query.limit.unwrap_or(200).clamp(1, 2000))
        .map_err(WebError::from)?;
    Ok(Json(history))
}

/// 读取指定会话的结构化轮次与工具时间线。
///
/// 参数:
/// - `state`: Web 应用状态
/// - `id`: 会话 ID
/// - `query`: 轮次数量限制
///
/// 返回:
/// - 会话时间线
async fn timeline(
    State(state): State<WebAppState>,
    Path(id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> WebResult<Json<Vec<crate::state::SessionTimelineTurn>>> {
    reject_during_run(&state).await?;
    crate::state::switch_session(&state.paths, &id)
        .map_err(|error| WebError::not_found(error.to_string()))?;
    let store = StateStore::new(&state.paths).map_err(WebError::from)?;
    let timeline = store
        .session_timeline(query.limit.unwrap_or(200).clamp(1, 2000))
        .map_err(WebError::from)?;
    Ok(Json(timeline))
}

/// 活动运行期间禁止切换会话状态。
async fn reject_during_run(state: &WebAppState) -> WebResult<()> {
    if state.runs.is_active().await {
        return Err(WebError::conflict(
            "stop the active agent run before changing session",
        ));
    }
    Ok(())
}
