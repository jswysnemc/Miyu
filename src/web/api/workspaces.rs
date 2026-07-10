use super::super::app_state::WebAppState;
use super::super::error::{WebError, WebResult};
use super::super::workspaces::WorkspaceInfo;
use axum::extract::{Path, State};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize)]
struct WorkspaceListResponse {
    active_id: String,
    workspaces: Vec<WorkspaceInfo>,
}

#[derive(Deserialize)]
struct AddWorkspaceRequest {
    path: String,
    name: Option<String>,
}

#[derive(Deserialize)]
struct RenameWorkspaceRequest {
    name: String,
}

#[derive(Serialize)]
struct RemovedResponse {
    removed: bool,
}

/// 返回工作区管理路由。
pub(super) fn routes() -> Router<WebAppState> {
    Router::new()
        .route("/api/workspaces", get(list).post(add))
        .route("/api/workspaces/:id", patch(rename).delete(remove))
        .route("/api/workspaces/:id/switch", post(switch))
}

/// 列出工作区和当前活动项。
async fn list(State(state): State<WebAppState>) -> WebResult<Json<WorkspaceListResponse>> {
    let active = state.workspaces.active().map_err(WebError::from)?;
    let workspaces = state.workspaces.list().map_err(WebError::from)?;
    Ok(Json(WorkspaceListResponse {
        active_id: active.id,
        workspaces,
    }))
}

/// 添加工作区。
async fn add(
    State(state): State<WebAppState>,
    Json(request): Json<AddWorkspaceRequest>,
) -> WebResult<Json<WorkspaceInfo>> {
    let workspace = state
        .workspaces
        .add(&PathBuf::from(request.path), request.name.as_deref())
        .map_err(|error| WebError::bad_request(error.to_string()))?;
    Ok(Json(workspace))
}

/// 重命名工作区。
async fn rename(
    State(state): State<WebAppState>,
    Path(id): Path<String>,
    Json(request): Json<RenameWorkspaceRequest>,
) -> WebResult<Json<WorkspaceInfo>> {
    let workspace = state
        .workspaces
        .rename(&id, &request.name)
        .map_err(|error| WebError::bad_request(error.to_string()))?;
    Ok(Json(workspace))
}

/// 切换活动工作区。
async fn switch(
    State(state): State<WebAppState>,
    Path(id): Path<String>,
) -> WebResult<Json<WorkspaceInfo>> {
    if state.runs.is_active().await {
        return Err(WebError::conflict(
            "stop the active agent run before switching workspace",
        ));
    }
    if state.terminals.has_sessions().map_err(WebError::from)? {
        return Err(WebError::conflict(
            "close active terminals before switching workspace",
        ));
    }
    let workspace = state
        .workspaces
        .switch(&id)
        .map_err(|error| WebError::bad_request(error.to_string()))?;
    Ok(Json(workspace))
}

/// 移除非活动工作区。
async fn remove(
    State(state): State<WebAppState>,
    Path(id): Path<String>,
) -> WebResult<Json<RemovedResponse>> {
    let removed = state
        .workspaces
        .remove(&id)
        .map_err(|error| WebError::bad_request(error.to_string()))?;
    Ok(Json(RemovedResponse { removed }))
}
