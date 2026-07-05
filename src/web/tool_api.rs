use super::app_state::WebAppState;
use super::error::ApiResult;
use crate::config::AppConfig;
use crate::tools::command::{list_background_tasks_for_user, read_background_task_output_for_user};
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct OutputQuery {
    #[serde(default = "default_stream")]
    stream: String,
    #[serde(default = "default_tail_lines")]
    tail_lines: usize,
}

/// 列出后台任务。
///
/// 参数:
/// - `state`: Web 应用共享状态
///
/// 返回:
/// - 后台任务 JSON
pub(crate) async fn list_background_tasks(
    State(state): State<WebAppState>,
) -> ApiResult<Json<serde_json::Value>> {
    AppConfig::init_files(&state.paths)?;
    let config = AppConfig::load_or_default(&state.paths)?;
    let raw = list_background_tasks_for_user(&state.paths, &config).await?;
    Ok(Json(serde_json::from_str(&raw)?))
}

/// 读取后台任务输出。
///
/// 参数:
/// - `state`: Web 应用共享状态
/// - `task_id`: 任务 ID
/// - `query`: 输出查询参数
///
/// 返回:
/// - 后台任务输出 JSON
pub(crate) async fn read_background_output(
    State(state): State<WebAppState>,
    Path(task_id): Path<String>,
    Query(query): Query<OutputQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    AppConfig::init_files(&state.paths)?;
    let config = AppConfig::load_or_default(&state.paths)?;
    let raw = read_background_task_output_for_user(
        &state.paths,
        &config,
        &task_id,
        &query.stream,
        query.tail_lines,
    )
    .await?;
    Ok(Json(serde_json::from_str(&raw)?))
}

/// 返回默认日志流。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 默认日志流
fn default_stream() -> String {
    "all".to_string()
}

/// 返回默认日志行数。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 默认日志行数
fn default_tail_lines() -> usize {
    120
}
