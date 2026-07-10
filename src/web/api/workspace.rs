use super::super::app_state::WebAppState;
use super::super::error::{WebError, WebResult};
use super::super::workspace;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

#[derive(Deserialize)]
struct TreeQuery {
    path: Option<String>,
    depth: Option<usize>,
}

#[derive(Deserialize)]
struct FileQuery {
    path: String,
}

#[derive(Deserialize)]
struct SaveFileRequest {
    path: String,
    content: String,
}

/// 返回工作区文件与 Diff 路由。
pub(super) fn routes() -> Router<WebAppState> {
    Router::new()
        .route("/api/workspace/tree", get(tree))
        .route("/api/workspace/file", get(file).put(save_file))
        .route("/api/workspace/diff", get(diff))
}

/// 读取文件树。
async fn tree(
    State(state): State<WebAppState>,
    Query(query): Query<TreeQuery>,
) -> WebResult<Json<Vec<workspace::FileNode>>> {
    let active = state.workspaces.active().map_err(WebError::from)?;
    let nodes = workspace::read_tree(
        std::path::Path::new(&active.path),
        query.path.as_deref().unwrap_or(""),
        query.depth.unwrap_or(4),
    )
    .map_err(|error| WebError::bad_request(error.to_string()))?;
    Ok(Json(nodes))
}

/// 读取文本文件。
async fn file(
    State(state): State<WebAppState>,
    Query(query): Query<FileQuery>,
) -> WebResult<Json<workspace::FileContent>> {
    let active = state.workspaces.active().map_err(WebError::from)?;
    let file = workspace::read_file(std::path::Path::new(&active.path), &query.path)
        .map_err(|error| WebError::bad_request(error.to_string()))?;
    Ok(Json(file))
}

/// 保存文本文件。
async fn save_file(
    State(state): State<WebAppState>,
    Json(request): Json<SaveFileRequest>,
) -> WebResult<Json<workspace::FileContent>> {
    let active = state.workspaces.active().map_err(WebError::from)?;
    let file = workspace::write_file(
        std::path::Path::new(&active.path),
        &request.path,
        &request.content,
    )
    .map_err(|error| WebError::bad_request(error.to_string()))?;
    Ok(Json(file))
}

/// 读取当前工作区 Git Diff。
async fn diff(State(state): State<WebAppState>) -> WebResult<Json<workspace::GitDiff>> {
    let active = state.workspaces.active().map_err(WebError::from)?;
    let diff = workspace::read_git_diff(std::path::Path::new(&active.path))
        .await
        .map_err(WebError::from)?;
    Ok(Json(diff))
}
