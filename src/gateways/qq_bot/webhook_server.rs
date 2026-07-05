use super::auth::QqBotAuthenticator;
use super::event::{parse_message_event, parse_validation_event, QqBotInboundMediaKind};
use super::inbound_media::{save_inbound_media, saved_image_to_data_url, SavedQqInboundMedia};
use super::prompt::channel_prompt;
use super::signature::{sign_validation, verify_event_signature};
use super::tools as qq_tools;
use crate::agent::{Agent, AgentMode};
use crate::cli::build_tool_registry;
use crate::config::AppConfig;
use crate::gateways::message::OutboundMessage;
use crate::gateways::qq_official::{QqOfficialClient, QqTargetKind};
use crate::llm::OpenAiCompatibleClient;
use crate::paths::MiyuPaths;
use crate::state::StateStore;
use anyhow::{Context, Result};
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

pub(crate) struct QqBotWebhookServerConfig {
    pub(crate) listen: SocketAddr,
    pub(crate) base_url: String,
    pub(crate) app_id: String,
    pub(crate) client_secret: String,
    pub(crate) verbose: bool,
}

struct QqBotWebhookState {
    paths: MiyuPaths,
    base_url: String,
    client_secret: String,
    verbose: bool,
    authenticator: Mutex<QqBotAuthenticator>,
    http_client: reqwest::Client,
    agent_lock: Mutex<()>,
}

impl QqBotWebhookState {
    /// 输出 QQ 网关调试日志。
    ///
    /// 参数:
    /// - `message`: 日志内容
    ///
    /// 返回:
    /// - 无
    fn debug_log(&self, message: impl AsRef<str>) {
        if self.verbose {
            eprintln!("【QQ网关】【调试】{}", message.as_ref());
        }
    }
}

/// 启动 QQ 官方机器人 Webhook 入站服务。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `config`: QQ Bot Webhook 服务配置
///
/// 返回:
/// - 服务运行结果
pub(crate) async fn run_qq_bot_webhook_server(
    paths: &MiyuPaths,
    config: QqBotWebhookServerConfig,
) -> Result<()> {
    let listen = config.listen;
    let state = Arc::new(QqBotWebhookState {
        paths: paths.clone(),
        base_url: config.base_url,
        client_secret: config.client_secret.clone(),
        verbose: config.verbose,
        authenticator: Mutex::new(QqBotAuthenticator::new(config.app_id, config.client_secret)),
        http_client: reqwest::Client::new(),
        agent_lock: Mutex::new(()),
    });
    let app = Router::new()
        .route("/", post(handle_qq_bot_webhook))
        .route("/qqbot", post(handle_qq_bot_webhook))
        .with_state(state.clone());
    let listener = TcpListener::bind(listen)
        .await
        .with_context(|| format!("failed to bind QQ Bot webhook server: {listen}"))?;
    println!("QQ Bot webhook server listening on http://{listen}");
    state.debug_log(format!("server started listen={listen}"));
    axum::serve(listener, app).await?;
    Ok(())
}

/// 接收 QQ 官方机器人 Webhook 请求。
///
/// 参数:
/// - `state`: 入站服务共享状态
/// - `headers`: HTTP 请求头
/// - `body`: 原始请求体
///
/// 返回:
/// - QQ Webhook HTTP 响应
async fn handle_qq_bot_webhook(
    State(state): State<Arc<QqBotWebhookState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    match handle_qq_bot_webhook_inner(state, headers, body).await {
        Ok(response) => response,
        Err(err) => {
            eprintln!("【QQ网关】【请求失败】{err:#}");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": err.to_string() })),
            )
                .into_response()
        }
    }
}

/// 处理 QQ 官方机器人 Webhook 请求。
///
/// 参数:
/// - `state`: 入站服务共享状态
/// - `headers`: HTTP 请求头
/// - `body`: 原始请求体
///
/// 返回:
/// - QQ Webhook HTTP 响应
async fn handle_qq_bot_webhook_inner(
    state: Arc<QqBotWebhookState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response> {
    let payload = serde_json::from_slice::<Value>(&body).with_context(|| "invalid QQ payload")?;
    if let Some(validation) = parse_validation_event(&payload)? {
        state.debug_log("收到回调地址验证事件");
        let signature = sign_validation(
            &state.client_secret,
            &validation.event_ts,
            &validation.plain_token,
        )?;
        return Ok(Json(json!({
            "plain_token": validation.plain_token,
            "signature": signature,
        }))
        .into_response());
    }
    verify_request_signature(&state.client_secret, &headers, &body)?;
    if let Some(event) = parse_message_event(&payload)? {
        state.debug_log(format!(
            "收到消息 event_type={} target_kind={} target_id={} media_count={}",
            event.event_type,
            target_kind_name(event.target_kind),
            event.target_id,
            event.media.len()
        ));
        tokio::spawn(async move {
            if let Err(err) = process_message_event(state, event).await {
                eprintln!("【QQ网关】【消息处理失败】{err:#}");
            }
        });
    }
    Ok(Json(json!({ "op": 12 })).into_response())
}

/// 校验 QQ Webhook 普通事件签名。
///
/// 参数:
/// - `client_secret`: QQ 开放平台 AppSecret
/// - `headers`: HTTP 请求头
/// - `body`: 原始请求体
///
/// 返回:
/// - 签名是否有效
fn verify_request_signature(client_secret: &str, headers: &HeaderMap, body: &[u8]) -> Result<()> {
    let signature = headers
        .get("X-Signature-Ed25519")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| anyhow::anyhow!("missing X-Signature-Ed25519"))?;
    let timestamp = headers
        .get("X-Signature-Timestamp")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| anyhow::anyhow!("missing X-Signature-Timestamp"))?;
    verify_event_signature(client_secret, timestamp, body, signature)
}

/// 处理一条 QQ 官方机器人消息事件。
///
/// 参数:
/// - `state`: 入站服务共享状态
/// - `event`: QQ 消息事件
///
/// 返回:
/// - 处理是否成功
async fn process_message_event(
    state: Arc<QqBotWebhookState>,
    event: super::event::QqBotMessageEvent,
) -> Result<()> {
    let _guard = state.agent_lock.lock().await;
    let (prompt, image_url) = prepare_agent_input(&state, &event).await?;
    let authorization = state.authenticator.lock().await.authorization().await?;
    let client = QqOfficialClient::new(
        state.base_url.clone(),
        authorization,
        event.target_kind,
        event.target_id.clone(),
    );
    let reply = run_agent(&state.paths, client.clone(), &event, prompt, image_url).await?;
    if reply.trim().is_empty() {
        return Ok(());
    }
    let message = OutboundMessage {
        text: Some(reply),
        media: Vec::new(),
    };
    client.send(&message, Some(&event.msg_id)).await?;
    Ok(())
}

/// 准备 Agent 输入文本和首张图片。
///
/// 参数:
/// - `state`: 入站服务共享状态
/// - `event`: QQ 消息事件
///
/// 返回:
/// - Agent 文本输入和可选图片 data URL
async fn prepare_agent_input(
    state: &QqBotWebhookState,
    event: &super::event::QqBotMessageEvent,
) -> Result<(String, Option<String>)> {
    let mut prompt = event.prompt.clone();
    let mut image_url = None;
    for media in &event.media {
        match save_inbound_media(&state.paths, &state.http_client, media).await {
            Ok(saved) => {
                append_saved_media_prompt(&mut prompt, &saved);
                state.debug_log(format!(
                    "入站附件已保存 kind={} name={} mime={} path={}",
                    inbound_media_name(saved.kind),
                    saved.name,
                    saved.mime_type,
                    saved.path.display()
                ));
                if saved.kind == QqBotInboundMediaKind::Image && image_url.is_none() {
                    match saved_image_to_data_url(&saved) {
                        Ok(data_url) => image_url = Some(data_url),
                        Err(err) => prompt.push_str(&format!("\n\n图片视觉输入准备失败: {err}")),
                    }
                }
            }
            Err(err) => {
                state.debug_log(format!(
                    "入站附件保存失败 kind={} source={} error={err:#}",
                    inbound_media_name(media.kind),
                    media.source
                ));
                prompt.push_str(&format!(
                    "\n\n用户发送的{}保存失败: {err}\n来源: {}",
                    inbound_media_label(media.kind),
                    media.source
                ));
            }
        }
    }
    Ok((prompt, image_url))
}

/// 将已保存媒体信息追加到 Agent 输入。
///
/// 参数:
/// - `prompt`: Agent 输入文本
/// - `saved`: 已保存媒体信息
///
/// 返回:
/// - 无
fn append_saved_media_prompt(prompt: &mut String, saved: &SavedQqInboundMedia) {
    prompt.push_str(&format!(
        "\n\n用户发送了{}: {}\n已保存到: {}\n媒体类型: {}",
        inbound_media_label(saved.kind),
        saved.name,
        saved.path.display(),
        saved.mime_type
    ));
}

/// 运行 Miyu Agent 并返回回复文本。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `qq_client`: QQ 官方机器人客户端
/// - `event`: QQ 消息事件
/// - `prompt`: Agent 输入文本
/// - `image_url`: 可选图片 data URL
///
/// 返回:
/// - Agent 回复文本
async fn run_agent(
    paths: &MiyuPaths,
    qq_client: QqOfficialClient,
    event: &super::event::QqBotMessageEvent,
    prompt: String,
    image_url: Option<String>,
) -> Result<String> {
    AppConfig::init_files(paths)?;
    let config = AppConfig::load_or_default(paths)?;
    let state = StateStore::new(paths)?;
    state.init_files()?;
    let client = OpenAiCompatibleClient::from_config(&config, paths)?;
    let mut registry = build_tool_registry(&config, paths, AgentMode::Yolo)?;
    qq_tools::register_channel_tools(&mut registry, qq_client, Some(event.msg_id.clone()));
    let progressive_loading_enabled = config.tools.progressive_loading_enabled;
    let mut agent = Agent::new_with_extra_system_prompt(
        config,
        paths,
        state.clone(),
        client,
        registry,
        AgentMode::Yolo,
        Some(channel_prompt()),
    )?;
    if progressive_loading_enabled {
        let mut loaded_tools = state.load_loaded_tools()?;
        loaded_tools.extend([
            "send_channel_image".to_string(),
            "send_channel_file".to_string(),
        ]);
        agent.restore_loaded_tools(&loaded_tools);
    }
    let result = agent
        .chat_stream_with_image(&prompt, image_url, |_| Ok(()))
        .await?;
    if progressive_loading_enabled {
        state.save_loaded_tools(&agent.loaded_tools())?;
    }
    Ok(result.content)
}

/// 返回 QQ 目标类型名称。
///
/// 参数:
/// - `kind`: QQ 目标类型
///
/// 返回:
/// - 类型名称
fn target_kind_name(kind: QqTargetKind) -> &'static str {
    match kind {
        QqTargetKind::User => "user",
        QqTargetKind::Group => "group",
    }
}

/// 返回入站媒体中文名称。
///
/// 参数:
/// - `kind`: 入站媒体类型
///
/// 返回:
/// - 媒体类型名称
fn inbound_media_label(kind: QqBotInboundMediaKind) -> &'static str {
    match kind {
        QqBotInboundMediaKind::Image => "图片",
        QqBotInboundMediaKind::Voice => "语音",
        QqBotInboundMediaKind::Video => "视频",
        QqBotInboundMediaKind::File => "文件",
    }
}

/// 返回入站媒体日志名称。
///
/// 参数:
/// - `kind`: 入站媒体类型
///
/// 返回:
/// - 媒体类型文本
fn inbound_media_name(kind: QqBotInboundMediaKind) -> &'static str {
    match kind {
        QqBotInboundMediaKind::Image => "image",
        QqBotInboundMediaKind::Voice => "voice",
        QqBotInboundMediaKind::Video => "video",
        QqBotInboundMediaKind::File => "file",
    }
}
