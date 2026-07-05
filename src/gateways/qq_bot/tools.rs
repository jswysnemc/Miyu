use crate::gateways::message::{MediaKind, OutboundMedia, OutboundMessage};
use crate::gateways::qq_official::QqOfficialClient;
use crate::tools::{ToolRegistry, ToolSpec};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

/// 注册 QQ 当前会话专用工具。
///
/// 参数:
/// - `registry`: 工具注册表
/// - `client`: QQ 官方机器人客户端
/// - `msg_id`: 当前入站消息 ID
///
/// 返回:
/// - 无
pub(crate) fn register_channel_tools(
    registry: &mut ToolRegistry,
    client: QqOfficialClient,
    msg_id: Option<String>,
) {
    register_media_tool(
        registry,
        "send_channel_image",
        "Send a local image file to the current QQ chat. Use this only for files that already exist on this machine.",
        MediaKind::Image,
        client.clone(),
        msg_id.clone(),
    );
    register_media_tool(
        registry,
        "send_channel_file",
        "Send a local file attachment to the current QQ chat. Use this only for files that already exist on this machine.",
        MediaKind::File,
        client,
        msg_id,
    );
}

/// 注册单个 QQ 媒体发送工具。
///
/// 参数:
/// - `registry`: 工具注册表
/// - `name`: 工具名称
/// - `description`: 工具描述
/// - `kind`: 媒体类型
/// - `client`: QQ 官方机器人客户端
/// - `msg_id`: 当前入站消息 ID
///
/// 返回:
/// - 无
fn register_media_tool(
    registry: &mut ToolRegistry,
    name: &'static str,
    description: &'static str,
    kind: MediaKind,
    client: QqOfficialClient,
    msg_id: Option<String>,
) {
    registry.register(
        ToolSpec::new(
            name,
            description,
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Local file path to send."
                    },
                    "caption": {
                        "type": "string",
                        "description": "Optional text sent before the media file."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            move |args| {
                let client = client.clone();
                let msg_id = msg_id.clone();
                async move { send_media_tool(args, client, msg_id, kind).await }
            },
        )
        .writes(),
    );
}

/// 执行 QQ 媒体发送工具。
///
/// 参数:
/// - `args`: 工具参数
/// - `client`: QQ 官方机器人客户端
/// - `msg_id`: 当前入站消息 ID
/// - `kind`: 媒体类型
///
/// 返回:
/// - JSON 格式发送结果
async fn send_media_tool(
    args: Value,
    client: QqOfficialClient,
    msg_id: Option<String>,
    kind: MediaKind,
) -> Result<String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .context("path is required")?;
    let caption = args
        .get("caption")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut responses = Vec::new();
    if let Some(caption) = caption {
        responses.extend(
            client
                .send(
                    &OutboundMessage {
                        text: Some(caption.to_string()),
                        media: Vec::new(),
                    },
                    msg_id.as_deref(),
                )
                .await?,
        );
    }
    responses.extend(
        client
            .send(
                &OutboundMessage {
                    text: None,
                    media: vec![OutboundMedia { kind, path }],
                },
                msg_id.as_deref(),
            )
            .await?,
    );
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "channel": "qq",
        "kind": media_kind_name(kind),
        "responses": responses,
    }))?)
}

/// 返回媒体类型名称。
///
/// 参数:
/// - `kind`: 媒体类型
///
/// 返回:
/// - 媒体类型文本
fn media_kind_name(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Image => "image",
        MediaKind::File => "file",
    }
}
