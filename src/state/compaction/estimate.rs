use crate::llm::ChatMessage;
use crate::state::turns::Turn;
use crate::token_estimate;

/// 估算即将发送给模型的消息上下文字符数（兼容旧路径）。
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

/// 估算即将发送给模型的消息上下文 token 数。
///
/// 优先对序列化后的消息体做 o200k_base 分词；序列化失败时回退到逐条文本估算。
///
/// 参数:
/// - `messages`: 当前请求消息列表
///
/// 返回:
/// - 粗略或 BPE token 数
pub fn estimate_chat_messages_tokens(messages: &[ChatMessage]) -> usize {
    if let Ok(serialized) = serde_json::to_string(messages) {
        return token_estimate::estimate_tokens(&serialized);
    }
    messages
        .iter()
        .map(|message| token_estimate::estimate_tokens(&format!("{message:?}")))
        .sum()
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
