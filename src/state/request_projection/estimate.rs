use super::model::{ProjectedRequest, ProjectionEstimate};
use crate::llm::ChatMessage;
use crate::state::compaction::estimate_chat_messages_chars;
use crate::state::session_snapshot;

/// 估算 provider turn 投影视图。
///
/// 参数:
/// - `messages`: 当前请求消息列表
/// - `context_limit_chars`: 当前模型上下文窗口字符数
///
/// 返回:
/// - provider 请求估算
pub(crate) fn project_provider_turn_estimate(
    messages: &[ChatMessage],
    context_limit_chars: usize,
) -> ProjectionEstimate {
    let message_chars = estimate_chat_messages_chars(messages);
    ProjectionEstimate {
        message_chars,
        state_context_chars: 0,
        context_limit_chars,
        context_ratio: session_snapshot::context_ratio(message_chars, context_limit_chars),
    }
}

/// 读取投影请求的 provider 消息字符估算。
///
/// 参数:
/// - `projection`: provider 请求投影视图
///
/// 返回:
/// - provider 消息字符估算
pub(crate) fn estimate_projected_request_chars(projection: &ProjectedRequest) -> usize {
    projection.estimate.message_chars
}
