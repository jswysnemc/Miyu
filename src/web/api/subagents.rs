use super::super::app_state::WebAppState;
use super::super::error::{WebError, WebResult};
use crate::tools::subagent_state::{cancel_subagent, list_subagents, SubagentSnapshot};
use axum::extract::Path;
use axum::routing::{get, post};
use axum::{Json, Router};

/// 返回子智能体管理路由。
pub(super) fn routes() -> Router<WebAppState> {
    Router::new()
        .route("/api/subagents", get(list))
        .route("/api/subagents/:id/cancel", post(cancel))
}

/// 列出当前进程内的子智能体。
async fn list() -> Json<Vec<SubagentSnapshot>> {
    Json(list_subagents())
}

/// 取消指定子智能体。
async fn cancel(Path(id): Path<String>) -> WebResult<Json<SubagentSnapshot>> {
    Ok(Json(
        cancel_subagent(&id).map_err(|error| WebError::not_found(error.to_string()))?,
    ))
}
