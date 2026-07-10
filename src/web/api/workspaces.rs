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
        .route("/api/workspaces/pick", post(pick))
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

/// 打开系统目录选择器并切换到所选工作区。
///
/// 参数:
/// - `state`: Web 应用状态
///
/// 返回:
/// - 选择并切换后的工作区，取消选择时返回空值
async fn pick(State(state): State<WebAppState>) -> WebResult<Json<Option<WorkspaceInfo>>> {
    ensure_workspace_switch_allowed(&state).await?;
    let selected = rfd::AsyncFileDialog::new()
        .set_title("选择 Miyu 工作区")
        .pick_folder()
        .await;
    let Some(selected) = selected else {
        return Ok(Json(None));
    };
    let workspace = state
        .workspaces
        .add(selected.path(), None)
        .map_err(|error| WebError::bad_request(error.to_string()))?;
    let workspace = state
        .workspaces
        .switch(&workspace.id)
        .map_err(|error| WebError::bad_request(error.to_string()))?;
    Ok(Json(Some(workspace)))
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
    ensure_workspace_switch_allowed(&state).await?;
    let workspace = state
        .workspaces
        .switch(&id)
        .map_err(|error| WebError::bad_request(error.to_string()))?;
    Ok(Json(workspace))
}

/// 校验当前状态是否允许切换工作区。
///
/// 参数:
/// - `state`: Web 应用状态
///
/// 返回:
/// - 允许切换时返回成功
async fn ensure_workspace_switch_allowed(state: &WebAppState) -> WebResult<()> {
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
    Ok(())
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
