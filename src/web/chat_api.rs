use super::app_state::WebAppState;
use super::chat_runner::{run_chat, ChatRunRequest};
use super::event_mapper::{encode_event, WebStreamEvent};
use super::session_store;
use crate::agent::AgentMode;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures_util::stream;
use serde::Deserialize;
use std::convert::Infallible;
use tokio::sync::mpsc;

#[derive(Debug, Deserialize)]
pub(crate) struct ChatRequest {
    session_id: Option<String>,
    message: String,
    image_url: Option<String>,
    mode: Option<String>,
    thinking: Option<String>,
}

/// 处理 Web 对话流请求。
///
/// 参数:
/// - `state`: Web 应用共享状态
/// - `request`: 对话请求
///
/// 返回:
/// - NDJSON 流响应
pub(crate) async fn chat(
    State(state): State<WebAppState>,
    Json(request): Json<ChatRequest>,
) -> Response {
    let session_id = match request.session_id {
        Some(id) if !id.trim().is_empty() => id,
        _ => match session_store::create_session(&state.paths) {
            Ok(session) => session.id,
            Err(err) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        },
    };
    let mode = match parse_mode(request.mode.as_deref()) {
        Ok(mode) => mode,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
    };
    let run_request = ChatRunRequest {
        session_id,
        message: request.message,
        image_url: request.image_url,
        mode,
        thinking: request.thinking,
    };
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let paths = state.paths.clone();
    tokio::spawn(async move {
        if let Err(err) = run_chat(paths, run_request, tx.clone()).await {
            let _ = tx.send(encode_event(&WebStreamEvent::Error {
                message: err.to_string(),
            }));
        }
    });
    let stream = stream::unfold(rx, |mut rx| async {
        rx.recv()
            .await
            .map(|line| (Ok::<Bytes, Infallible>(Bytes::from(line)), rx))
    });
    (
        [
            (header::CONTENT_TYPE, "application/x-ndjson; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        Body::from_stream(stream),
    )
        .into_response()
}

/// 解析 Agent 模式。
///
/// 参数:
/// - `value`: 模式文本
///
/// 返回:
/// - Agent 模式
fn parse_mode(value: Option<&str>) -> Result<AgentMode, String> {
    match value.unwrap_or("yolo").trim().to_ascii_lowercase().as_str() {
        "" | "yolo" => Ok(AgentMode::Yolo),
        "plan" => Ok(AgentMode::Plan),
        value => Err(format!("unsupported mode: {value}")),
    }
}

/// 返回 JSON 错误响应。
///
/// 参数:
/// - `status`: HTTP 状态
/// - `message`: 错误消息
///
/// 返回:
/// - HTTP 响应
fn json_error(status: StatusCode, message: String) -> Response {
    (
        status,
        Json(serde_json::json!({
            "ok": false,
            "error": message,
        })),
    )
        .into_response()
}
