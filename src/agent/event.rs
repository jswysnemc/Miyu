use crate::llm::{ChatStreamChunk, ToolCallStreamProgress};

/// Agent 向 CLI、TUI 与 Web 发送的统一运行事件。
#[derive(Debug, Clone)]
pub enum AgentEvent {
    Chunk(ChatStreamChunk),
    ToolCall {
        name: String,
        arguments: String,
    },
    ToolCallProgress(ToolCallStreamProgress),
    ToolResult {
        name: String,
        ok: bool,
        output: String,
    },
    ToolProgress {
        name: String,
        message: String,
    },
    PermissionRequested(crate::permission::PermissionRequest),
    PermissionResolved {
        request_id: String,
        decision: crate::permission::PermissionDecision,
    },
    CompactionStarted {
        turn_count: usize,
    },
    CompactionFinished {
        applied: bool,
    },
    FlushContent,
    ExternalOutput,
}
