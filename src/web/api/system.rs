use super::super::app_state::WebAppState;
use super::super::error::{WebError, WebResult};
use crate::config::AppConfig;
use crate::state::StateStore;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct SystemUsageResponse {
    session: SessionUsageResponse,
    process: ProcessUsageResponse,
    runtime: RuntimeUsageResponse,
}

#[derive(Serialize)]
struct SessionUsageResponse {
    id: String,
    requests: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    turn_count: usize,
    context_prompt_tokens: usize,
    context_window_tokens: usize,
    context_token_ratio: f32,
    tool_calls: usize,
    checkpoint_count: usize,
    compacted_turns: usize,
    latest_checkpoint_at: Option<String>,
    latest_checkpoint_reason: Option<String>,
    compaction_warning: Option<String>,
}

#[derive(Serialize)]
struct ProcessUsageResponse {
    pid: u32,
    uptime_seconds: u64,
    rss_bytes: Option<u64>,
    cpu_percent: f64,
}

#[derive(Serialize)]
struct RuntimeUsageResponse {
    active_run: bool,
    terminal_count: usize,
}

/// 返回系统用量路由。
///
/// 返回:
/// - 系统用量 API 路由
pub(super) fn routes() -> Router<WebAppState> {
    Router::new().route("/api/system/usage", get(usage))
}

/// 聚合当前会话、进程和 Web 运行时用量。
///
/// 参数:
/// - `state`: Web 应用状态
///
/// 返回:
/// - 系统用量快照
async fn usage(State(state): State<WebAppState>) -> WebResult<Json<SystemUsageResponse>> {
    let config = AppConfig::load_or_default(&state.paths).map_err(WebError::from)?;
    let context_window_tokens = config.active_context_chars().map_err(WebError::from)?;
    let store = StateStore::new(&state.paths).map_err(WebError::from)?;
    let snapshot = store
        .session_snapshot(context_window_tokens)
        .map_err(WebError::from)?;
    let process = state.system_monitor.snapshot();
    let terminal_count = state.terminals.list().map_err(WebError::from)?.len();
    let workspace = state.workspaces.active().map_err(WebError::from)?;
    let active_run = state
        .runs
        .is_session_active(&workspace.id, &snapshot.session_id)
        .await;
    Ok(Json(SystemUsageResponse {
        session: SessionUsageResponse {
            id: snapshot.session_id,
            requests: snapshot.usage.requests,
            prompt_tokens: snapshot.usage.prompt_tokens,
            completion_tokens: snapshot.usage.completion_tokens,
            total_tokens: snapshot.usage.total_tokens,
            turn_count: snapshot.turn_count,
            context_prompt_tokens: snapshot.context_prompt_tokens,
            context_window_tokens: snapshot.context_window_tokens,
            context_token_ratio: snapshot.context_token_ratio,
            tool_calls: snapshot.tool_history.call_count,
            checkpoint_count: snapshot.checkpoint_count,
            compacted_turns: snapshot.checkpoint_covered_turns,
            latest_checkpoint_at: snapshot.latest_checkpoint_at,
            latest_checkpoint_reason: snapshot.latest_checkpoint_reason,
            compaction_warning: snapshot
                .projection_warnings
                .iter()
                .find(|warning| warning.contains("多次压缩"))
                .cloned(),
        },
        process: ProcessUsageResponse {
            pid: process.pid,
            uptime_seconds: process.uptime_seconds,
            rss_bytes: process.rss_bytes,
            cpu_percent: process.cpu_percent,
        },
        runtime: RuntimeUsageResponse {
            active_run,
            terminal_count,
        },
    }))
}
