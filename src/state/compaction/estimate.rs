use crate::llm::ChatMessage;
use crate::state::turns::{Turn, TurnStatus};

/// 估算即将发送给模型的消息上下文字符数。
///
/// 参数:
/// - `messages`: 当前请求消息列表
///
/// 返回:
/// - JSON 序列化后的字符数量估算
pub fn estimate_chat_messages_chars(messages: &[ChatMessage]) -> usize {
    serde_json::to_string(messages)
        .map(|value| value.chars().count())
        .unwrap_or_else(|_| {
            messages
                .iter()
                .map(|message| format!("{message:?}").chars().count())
                .sum()
        })
}

/// 估算当前会话状态注入上下文后的字符数。
///
/// 参数:
/// - `turns`: 当前会话轮次
/// - `summary`: 可选压缩摘要
///
/// 返回:
/// - 上下文字符数量估算
pub fn estimate_state_context_chars(turns: &[Turn], summary: Option<&str>) -> usize {
    let summary_chars = summary
        .map(|value| value.trim().chars().count())
        .unwrap_or_default();
    summary_chars
        + turns
            .iter()
            .filter(|turn| turn.status != TurnStatus::Running)
            .map(estimate_turn_context_chars)
            .sum::<usize>()
}

/// 估算单个轮次在对话上下文中的字符数。
///
/// 参数:
/// - `turn`: 对话轮次
///
/// 返回:
/// - 轮次上下文字符数量估算
pub fn estimate_turn_context_chars(turn: &Turn) -> usize {
    turn.user_content.chars().count()
        + turn.assistant_content.chars().count()
        + turn
            .assistant_reasoning
            .as_deref()
            .map(|value| value.chars().count())
            .unwrap_or_default()
        + turn
            .tool_reports
            .iter()
            .map(|report| report.chars().count())
            .sum::<usize>()
}
