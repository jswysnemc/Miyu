use super::{Agent, AgentEvent};
use crate::llm::{ChatMessage, OpenAiCompatibleClient};
use crate::perf_trace::PerfTrace;
use crate::state::request_projection::{project_provider_turn_from_messages, ProjectedRequest};
use crate::state::{
    CompactionApplyOutcome, CompactionRequest, FailureKind, RecoveryStatus, StateStore,
};
use anyhow::Result;

impl Agent {
    /// 按当前请求上下文估算自动压缩旧会话。
    ///
    /// 参数:
    /// - `turn_id`: 当前运行中轮次标识
    /// - `messages`: 当前即将发送给模型的消息列表
    ///
    /// 返回:
    /// - 是否执行了压缩
    pub(super) async fn compact_conversation_if_needed(
        &self,
        turn_id: &str,
        messages: &[ChatMessage],
        on_event: &mut impl FnMut(AgentEvent) -> Result<()>,
    ) -> Result<bool> {
        let projection = project_provider_turn_from_messages(messages, 0, self.context_tokens);
        if !self.state.should_attempt_auto_compaction()? {
            return Ok(false);
        }
        let Some(request) = self.state.select_compaction_for_projection(
            &projection,
            self.trim_at_ratio,
            self.config.context.trim_batch_ratio,
        )?
        else {
            return Ok(false);
        };
        on_event(AgentEvent::CompactionStarted {
            turn_count: request.turn_count(),
        })?;
        let summary = match self.create_compaction_summary(&request).await {
            Ok(summary) => summary,
            Err(err) => {
                self.state.record_auto_compaction_failure(
                    request.compact_turn_ids.last().map(String::as_str),
                    classify_compaction_error_for_request(&request, &err),
                    &format!("{err:#}"),
                    projection.estimate.message_chars,
                    projection.estimate.context_limit_chars,
                )?;
                on_event(AgentEvent::CompactionFinished { applied: false })?;
                return Ok(false);
            }
        };
        match self.state.apply_compaction_with_budget_guard(
            &request,
            &summary,
            &projection,
            Some(turn_id),
        )? {
            CompactionApplyOutcome::Applied => {
                on_event(AgentEvent::CompactionFinished { applied: true })?;
                Ok(true)
            }
            CompactionApplyOutcome::RejectedOverBudget if request.is_memory_first() => {
                on_event(AgentEvent::CompactionFinished { applied: false })?;
                self.compact_with_legacy_fallback(turn_id, &projection, on_event)
                    .await
            }
            CompactionApplyOutcome::RejectedOverBudget => {
                on_event(AgentEvent::CompactionFinished { applied: false })?;
                Ok(false)
            }
        }
    }

    /// memory-first compact 被拒绝后尝试传统 checkpoint compact。
    ///
    /// 参数:
    /// - `turn_id`: 当前运行中轮次标识
    /// - `projection`: 当前 provider 请求投影视图
    ///
    /// 返回:
    /// - 是否执行了压缩
    async fn compact_with_legacy_fallback(
        &self,
        turn_id: &str,
        projection: &ProjectedRequest,
        on_event: &mut impl FnMut(AgentEvent) -> Result<()>,
    ) -> Result<bool> {
        let Some(request) = self.state.select_legacy_compaction_for_projection(
            projection,
            self.trim_at_ratio,
            self.config.context.trim_batch_ratio,
        )?
        else {
            return Ok(false);
        };
        on_event(AgentEvent::CompactionStarted {
            turn_count: request.turn_count(),
        })?;
        let summary = match self.create_compaction_summary(&request).await {
            Ok(summary) => summary,
            Err(err) => {
                self.state.record_auto_compaction_failure(
                    request.compact_turn_ids.last().map(String::as_str),
                    classify_compaction_error_for_request(&request, &err),
                    &format!("{err:#}"),
                    projection.estimate.message_chars,
                    projection.estimate.context_limit_chars,
                )?;
                on_event(AgentEvent::CompactionFinished { applied: false })?;
                return Ok(false);
            }
        };
        match self.state.apply_compaction_with_budget_guard(
            &request,
            &summary,
            projection,
            Some(turn_id),
        )? {
            CompactionApplyOutcome::Applied => {
                on_event(AgentEvent::CompactionFinished { applied: true })?;
                Ok(true)
            }
            CompactionApplyOutcome::RejectedOverBudget => {
                on_event(AgentEvent::CompactionFinished { applied: false })?;
                Ok(false)
            }
        }
    }

    /// provider 上下文溢出后尝试一次压缩恢复。
    ///
    /// 参数:
    /// - `turn_id`: 当前轮次标识
    /// - `messages`: 触发溢出的 provider 消息
    /// - `err`: provider 错误
    /// - `input`: 当前用户输入
    /// - `image_url`: 可选图片 data URL
    /// - `association_prompt`: 可选关联记忆上下文
    /// - `auto_meme_reminder`: 可选自动表情包提醒
    ///
    /// 返回:
    /// - 是否已经压缩并允许重试
    pub(super) async fn recover_after_provider_overflow(
        &mut self,
        turn_id: &str,
        messages: &[ChatMessage],
        err: &anyhow::Error,
        input: &str,
        image_url: Option<&str>,
        association_prompt: Option<&str>,
        auto_meme_reminder: Option<&str>,
    ) -> Result<bool> {
        let projection = project_provider_turn_from_messages(messages, 0, self.context_tokens);
        self.state.record_provider_overflow_recovery(
            Some(turn_id),
            FailureKind::ProviderOverflow,
            RecoveryStatus::Recovering,
            &format!("{err:#}"),
            projection.estimate.message_chars,
            projection.estimate.context_limit_chars,
        )?;
        let Some(request) = self.state.select_compaction_for_projection(
            &projection,
            0.0,
            self.config.context.trim_batch_ratio,
        )?
        else {
            self.record_overflow_retry_failed(turn_id, messages, err)?;
            return Ok(false);
        };
        let summary = match self.create_compaction_summary(&request).await {
            Ok(summary) => summary,
            Err(compaction_err) => {
                self.record_overflow_retry_failed(turn_id, messages, &compaction_err)?;
                return Ok(false);
            }
        };
        let budget =
            self.state
                .compaction_budget_check(&request, &summary, &projection, Some(turn_id))?;
        if budget.is_over_budget() {
            self.state.record_provider_overflow_recovery(
                Some(turn_id),
                FailureKind::OverflowRetryFailed,
                RecoveryStatus::Terminal,
                &format!(
                    "provider overflow recovery compaction over budget: result_chars={}, context_limit_chars={}",
                    budget.result_chars, budget.context_limit_chars
                ),
                budget.context_chars,
                budget.context_limit_chars,
            )?;
            return Ok(false);
        }
        self.state.apply_compaction(&request, &summary)?;
        let reprojected = self.chat_messages_for_turn(
            turn_id,
            input,
            image_url,
            association_prompt,
            auto_meme_reminder,
        )?;
        let reprojected_projection =
            project_provider_turn_from_messages(&reprojected, 0, self.context_tokens);
        self.state.record_provider_overflow_recovery(
            Some(turn_id),
            FailureKind::ProviderOverflow,
            RecoveryStatus::Reprojected,
            "provider overflow recovery compacted history and rebuilt request projection",
            reprojected_projection.estimate.message_chars,
            reprojected_projection.estimate.context_limit_chars,
        )?;
        Ok(true)
    }

    /// 记录 provider overflow 重试失败。
    ///
    /// 参数:
    /// - `turn_id`: 当前轮次标识
    /// - `messages`: 失败时 provider 消息
    /// - `err`: provider 错误
    ///
    /// 返回:
    /// - 写入是否成功
    pub(super) fn record_overflow_retry_failed(
        &self,
        turn_id: &str,
        messages: &[ChatMessage],
        err: &anyhow::Error,
    ) -> Result<()> {
        let projection = project_provider_turn_from_messages(messages, 0, self.context_tokens);
        self.state.record_provider_overflow_recovery(
            Some(turn_id),
            FailureKind::OverflowRetryFailed,
            RecoveryStatus::Terminal,
            &format!("{err:#}"),
            projection.estimate.message_chars,
            projection.estimate.context_limit_chars,
        )
    }

    /// 后台启动 Session Memory 模型提取。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    pub(super) fn spawn_session_memory_extraction(&self) {
        let state = self.state.clone();
        let client = self.client.clone();
        let context_tokens = self.context_tokens;
        tokio::spawn(async move {
            let _ = extract_session_memory_with_model(state, client, context_tokens).await;
        });
    }
}

/// 使用独立模型请求提取 Session Memory。
///
/// 参数:
/// - `state`: 状态仓储
/// - `client`: 模型客户端
/// - `context_tokens`: 当前主模型上下文窗口字符数
///
/// 返回:
/// - 提取是否成功
async fn extract_session_memory_with_model(
    state: StateStore,
    client: OpenAiCompatibleClient,
    context_tokens: usize,
) -> Result<bool> {
    let mut perf = PerfTrace::new("session-memory");
    perf.mark("start");
    let Some(input) = state.prepare_session_memory_model_extraction(context_tokens)? else {
        perf.mark("skip");
        return Ok(false);
    };
    perf.mark("prepared");
    let messages = vec![
        ChatMessage::system(
            "You are a session memory extraction worker. Update durable conversation memory only. Do not answer the user task.",
        ),
        ChatMessage::plain("user", input.prompt.clone()),
    ];
    let result = match client
        .chat_stream_events(messages, Vec::new(), |_| Ok(()))
        .await
    {
        Ok(result) => result,
        Err(err) => {
            state.record_session_memory_model_extraction_failure(&input, &format!("{err:#}"))?;
            perf.mark("failed");
            return Ok(false);
        }
    };
    perf.mark("model done");
    if let Some(usage) = &result.usage {
        state.add_auxiliary_usage(usage)?;
    }
    state.apply_session_memory_model_extraction(&input, &result.content)?;
    perf.mark("applied");
    Ok(true)
}

/// 识别压缩失败类型。
///
/// 参数:
/// - `err`: 压缩错误
///
/// 返回:
/// - 失败类型
pub(super) fn classify_compaction_error(err: &anyhow::Error) -> FailureKind {
    let message = format!("{err:#}");
    if message.contains("compaction summary is empty") {
        FailureKind::EmptySummary
    } else if message.contains("tool history summary prompt over budget") {
        FailureKind::ToolHistoryPromptOverBudget
    } else {
        FailureKind::CompactionLlmFailed
    }
}

/// 结合压缩请求识别压缩失败类型。
///
/// 参数:
/// - `request`: 压缩请求
/// - `err`: 压缩错误
///
/// 返回:
/// - 失败类型
pub(super) fn classify_compaction_error_for_request(
    request: &CompactionRequest,
    err: &anyhow::Error,
) -> FailureKind {
    let kind = classify_compaction_error(err);
    if kind == FailureKind::ToolHistoryPromptOverBudget {
        return kind;
    }
    if request.is_memory_first() {
        FailureKind::SessionMemoryCompactFailed
    } else {
        kind
    }
}

/// 判断 provider 错误是否属于上下文溢出。
///
/// 参数:
/// - `err`: provider 错误
///
/// 返回:
/// - 是否属于上下文溢出
pub(super) fn is_context_overflow_error(err: &anyhow::Error) -> bool {
    let message = format!("{err:#}").to_ascii_lowercase();
    [
        "context_length_exceeded",
        "maximum context",
        "context window",
        "context length",
        "too many tokens",
        "tokens exceed",
        "prompt is too long",
        "input is too long",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CompactionRequest, Turn, TurnStatus};

    fn completed_turn(seq: i64) -> Turn {
        Turn {
            turn_id: format!("turn_{seq}"),
            seq,
            user_content: format!("user {seq}"),
            user_timestamp: "2026-01-01T00:00:00Z".to_string(),
            assistant_content: format!("assistant {seq}"),
            assistant_reasoning: None,
            assistant_timestamp: Some("2026-01-01T00:00:01Z".to_string()),
            status: TurnStatus::Completed,
            tool_reports: Vec::new(),
        }
    }

    #[test]
    fn classifies_empty_compaction_summary() {
        let err = anyhow::anyhow!("compaction summary is empty");

        assert_eq!(classify_compaction_error(&err), FailureKind::EmptySummary);
    }

    #[test]
    fn classifies_memory_first_compaction_error() {
        let request = CompactionRequest::new(vec![completed_turn(3)], Some("memory".to_string()))
            .with_coverage(1, 3, 3, 3);
        let err = anyhow::anyhow!("provider error");

        assert_eq!(
            classify_compaction_error_for_request(&request, &err),
            FailureKind::SessionMemoryCompactFailed
        );
    }

    #[test]
    fn detects_context_overflow_errors() {
        let err = anyhow::anyhow!(
            "chat completions stream request failed (400): context_length_exceeded"
        );

        assert!(is_context_overflow_error(&err));
    }

    #[test]
    fn ignores_non_overflow_provider_errors() {
        let err = anyhow::anyhow!("chat completions stream request failed (401): invalid key");

        assert!(!is_context_overflow_error(&err));
    }
}
