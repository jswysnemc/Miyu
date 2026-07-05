use crate::agent::AgentEvent;
use crate::llm::{ChatStreamKind, ToolCallStreamProgress, Usage};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum WebStreamEvent {
    Session {
        session_id: String,
    },
    Chunk {
        kind: &'static str,
        text: String,
    },
    ToolCall {
        name: String,
        arguments: String,
    },
    ToolCallProgress {
        index: usize,
        name: Option<String>,
        arguments_chars: usize,
        arguments_bytes: usize,
        arguments_preview: String,
    },
    ToolProgress {
        name: String,
        message: String,
    },
    ToolResult {
        name: String,
        ok: bool,
        output: String,
    },
    Done {
        content: String,
        reasoning: Option<String>,
        usage: Option<Usage>,
        loaded_tools: Vec<String>,
    },
    Error {
        message: String,
    },
}

/// 将 Agent 事件转换为 Web 流事件。
///
/// 参数:
/// - `event`: Agent 事件
///
/// 返回:
/// - Web 流事件
pub(crate) fn agent_event_to_web(event: AgentEvent) -> WebStreamEvent {
    match event {
        AgentEvent::Chunk(chunk) => WebStreamEvent::Chunk {
            kind: chunk_kind_name(chunk.kind),
            text: chunk.text,
        },
        AgentEvent::ToolCall { name, arguments } => WebStreamEvent::ToolCall { name, arguments },
        AgentEvent::ToolCallProgress(progress) => tool_progress_event(progress),
        AgentEvent::ToolResult { name, ok, output } => {
            WebStreamEvent::ToolResult { name, ok, output }
        }
        AgentEvent::ToolProgress { name, message } => {
            WebStreamEvent::ToolProgress { name, message }
        }
        AgentEvent::ExternalOutput => WebStreamEvent::ToolProgress {
            name: "external_output".to_string(),
            message: "外部输出已写入终端".to_string(),
        },
    }
}

/// 将事件编码为 NDJSON 行。
///
/// 参数:
/// - `event`: Web 流事件
///
/// 返回:
/// - NDJSON 文本行
pub(crate) fn encode_event(event: &WebStreamEvent) -> String {
    format!(
        "{}\n",
        serde_json::to_string(event).unwrap_or_else(|err| {
            format!(r#"{{"type":"error","message":"failed to encode event: {err}"}}"#)
        })
    )
}

/// 返回流块类型名称。
///
/// 参数:
/// - `kind`: 流块类型
///
/// 返回:
/// - 类型名称
fn chunk_kind_name(kind: ChatStreamKind) -> &'static str {
    match kind {
        ChatStreamKind::Content => "content",
        ChatStreamKind::Reasoning => "reasoning",
    }
}

/// 转换工具调用进度事件。
///
/// 参数:
/// - `progress`: 工具调用进度
///
/// 返回:
/// - Web 流事件
fn tool_progress_event(progress: ToolCallStreamProgress) -> WebStreamEvent {
    WebStreamEvent::ToolCallProgress {
        index: progress.index,
        name: progress.name,
        arguments_chars: progress.arguments_chars,
        arguments_bytes: progress.arguments_bytes,
        arguments_preview: progress.arguments_preview,
    }
}
