use super::client::WeixinBotClient;
use super::media::{send_local_media, WeixinOutboundMediaKind};
use crate::tools::{ToolRegistry, ToolSpec};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

/// 注册微信当前会话专用工具。
///
/// 参数:
/// - `registry`: 工具注册表
/// - `client`: 微信 iLink 客户端
/// - `to_user_id`: 当前会话接收方用户 ID
/// - `context_token`: 当前入站消息上下文 token
///
/// 返回:
/// - 无
pub(crate) fn register_channel_tools(
    registry: &mut ToolRegistry,
    client: WeixinBotClient,
    to_user_id: String,
    context_token: Option<String>,
) {
    register_media_tool(
        registry,
        "send_channel_image",
        "Send a local image file to the current Weixin chat. Use this only for files that already exist on this machine.",
        WeixinOutboundMediaKind::Image,
        client.clone(),
        to_user_id.clone(),
        context_token.clone(),
    );
    register_media_tool(
        registry,
        "send_channel_file",
        "Send a local file attachment to the current Weixin chat. Use this only for files that already exist on this machine.",
        WeixinOutboundMediaKind::File,
        client.clone(),
        to_user_id.clone(),
        context_token.clone(),
    );
    register_media_tool(
        registry,
        "send_channel_video",
        "Send a local video file to the current Weixin chat. Use this only for files that already exist on this machine.",
        WeixinOutboundMediaKind::Video,
        client,
        to_user_id,
        context_token,
    );
}

/// 注册单个微信媒体发送工具。
///
/// 参数:
/// - `registry`: 工具注册表
/// - `name`: 工具名称
/// - `description`: 工具描述
/// - `kind`: 媒体类型
/// - `client`: 微信 iLink 客户端
/// - `to_user_id`: 当前会话接收方用户 ID
/// - `context_token`: 当前入站消息上下文 token
///
/// 返回:
/// - 无
fn register_media_tool(
    registry: &mut ToolRegistry,
    name: &'static str,
    description: &'static str,
    kind: WeixinOutboundMediaKind,
    client: WeixinBotClient,
    to_user_id: String,
    context_token: Option<String>,
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
                let to_user_id = to_user_id.clone();
                let context_token = context_token.clone();
                async move { send_media_tool(args, client, to_user_id, context_token, kind).await }
            },
        )
        .writes(),
    );
}

/// 执行微信媒体发送工具。
///
/// 参数:
/// - `args`: 工具参数
/// - `client`: 微信 iLink 客户端
/// - `to_user_id`: 当前会话接收方用户 ID
/// - `context_token`: 当前入站消息上下文 token
/// - `kind`: 媒体类型
///
/// 返回:
/// - JSON 格式发送结果
async fn send_media_tool(
    args: Value,
    client: WeixinBotClient,
    to_user_id: String,
    context_token: Option<String>,
    kind: WeixinOutboundMediaKind,
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
    let message_id = send_local_media(
        &client,
        &to_user_id,
        context_token.as_deref(),
        &path,
        caption,
        kind,
    )
    .await?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "channel": "weixin",
        "kind": media_kind_name(kind),
        "message_id": message_id,
    }))?)
}

/// 返回媒体类型名称。
///
/// 参数:
/// - `kind`: 媒体类型
///
/// 返回:
/// - 媒体类型文本
fn media_kind_name(kind: WeixinOutboundMediaKind) -> &'static str {
    match kind {
        WeixinOutboundMediaKind::Image => "image",
        WeixinOutboundMediaKind::Video => "video",
        WeixinOutboundMediaKind::File => "file",
    }
}
