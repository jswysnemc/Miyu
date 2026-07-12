use super::super::app_state::WebAppState;
use super::super::error::{WebError, WebResult};
use crate::tools::subagent_state::{
    cancel_subagent, list_subagents, subagent_snapshot, subagent_timeline, SubagentSnapshot,
};
use axum::extract::Path;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::Value;

/// 返回子智能体管理路由。
pub(super) fn routes() -> Router<WebAppState> {
    Router::new()
        .route("/api/subagents", get(list))
        .route("/api/subagents/:id", get(detail))
        .route("/api/subagents/:id/cancel", post(cancel))
}

/// 列出当前进程内的子智能体。
async fn list() -> Json<Vec<SubagentSnapshot>> {
    Json(list_subagents())
}

/// 返回单个子智能体的详情,附带执行时间线。
async fn detail(Path(id): Path<String>) -> WebResult<Json<Value>> {
    let snapshot =
        subagent_snapshot(&id).map_err(|error| WebError::not_found(error.to_string()))?;
    let timeline =
        subagent_timeline(&id).map_err(|error| WebError::not_found(error.to_string()))?;
    // 1. 快照字段平铺,时间线作为附加字段合并进同一响应
    let mut body = serde_json::to_value(&snapshot).map_err(anyhow::Error::from)?;
    if let Value::Object(map) = &mut body {
        map.insert(
            "timeline".to_string(),
            serde_json::to_value(&timeline).map_err(anyhow::Error::from)?,
        );
    }
    Ok(Json(body))
}

/// 取消指定子智能体。
async fn cancel(Path(id): Path<String>) -> WebResult<Json<SubagentSnapshot>> {
    Ok(Json(
        cancel_subagent(&id).map_err(|error| WebError::not_found(error.to_string()))?,
    ))
}

#[cfg(test)]
mod tests {
    use crate::tools::subagent_state::create_subagent;

    #[tokio::test]
    async fn detail_returns_snapshot_with_timeline() {
        let (subagent, _cancel) =
            create_subagent("detail api".to_string(), "general".to_string(), 5);

        let response = super::detail(axum::extract::Path(subagent.id.clone()))
            .await
            .unwrap();
        let body = response.0;

        assert_eq!(body["id"], subagent.id.as_str());
        assert!(body["timeline"].is_array());
    }

    #[tokio::test]
    async fn detail_rejects_unknown_id() {
        let result = super::detail(axum::extract::Path("missing-subagent".to_string())).await;

        assert!(result.is_err());
    }
}
