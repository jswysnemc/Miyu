use super::Agent;
use crate::llm::ChatMessage;
use crate::perf_trace::PerfTrace;
use crate::state::{CompactionApplyOutcome, CompactionRequest};
use anyhow::{bail, Context, Result};

impl Agent {
    /// 在工具轮次之间按需压缩，并重建当前运行轮次消息。
    ///
    /// 参数:
    /// - `tool_round`: 当前工具轮次
    /// - `turn_id`: 当前运行轮次标识
    /// - `messages`: 当前内存消息列表
    /// - `input`: 当前用户输入
    /// - `image_urls`: 当前用户图片
    /// - `association_prompt`: 关联记忆上下文
    /// - `auto_meme_reminder`: 自动表情提醒
    /// - `on_event`: 运行事件回调
    /// - `perf`: 性能追踪器
    ///
    /// 返回:
    /// - 是否应用了中途压缩
    pub(super) async fn compact_between_tool_rounds(
        &mut self,
        tool_round: usize,
        turn_id: &str,
        messages: &mut Vec<ChatMessage>,
        input: &str,
        image_urls: &[String],
        association_prompt: Option<&str>,
        auto_meme_reminder: Option<&str>,
        on_event: &mut impl FnMut(super::AgentEvent) -> Result<()>,
        perf: &mut PerfTrace,
    ) -> Result<bool> {
        if tool_round <= 1
            || !self
                .compact_conversation_if_needed(turn_id, messages, on_event)
                .await?
        {
            return Ok(false);
        }
        let trailing_runtime_messages = messages
            .iter()
            .rev()
            .take_while(|message| message.role == "system")
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev();
        *messages = self.chat_messages_for_turn(
            turn_id,
            input,
            image_urls,
            association_prompt,
            auto_meme_reminder,
        )?;
        messages.extend(self.state.project_running_turn_tool_messages(turn_id)?);
        messages.extend(trailing_runtime_messages);
        perf.mark(&format!(
            "round {tool_round} rebuilt after mid-turn compaction"
        ));
        Ok(true)
    }

    /// 立即手动压缩当前会话旧轮次。
    ///
    /// 参数:
    /// - `keep_tail_turns`: 保留的最近非运行轮次数量
    ///
    /// 返回:
    /// - 已压缩轮次数量
    pub async fn compact_conversation_now(&self, keep_tail_turns: usize) -> Result<usize> {
        let Some(request) = self.state.select_manual_compaction(keep_tail_turns)? else {
            return Ok(0);
        };
        let turn_count = request.turn_count();
        let summary = match self.create_compaction_summary(&request).await {
            Ok(summary) => summary,
            Err(err) => {
                self.state.record_manual_compaction_failure(
                    super::recovery::classify_compaction_error(&err),
                    &format!("{err:#}"),
                    request.prompt().chars().count(),
                    self.context_tokens,
                )?;
                return Err(err).context("failed to compact conversation");
            }
        };
        match self.state.apply_manual_compaction_with_budget_guard(
            &request,
            &summary,
            self.context_tokens,
        )? {
            CompactionApplyOutcome::Applied => Ok(turn_count),
            CompactionApplyOutcome::RejectedOverBudget => {
                bail!("manual compaction result exceeds context budget")
            }
        }
    }

    /// 使用当前模型生成会话压缩摘要。
    ///
    /// 参数:
    /// - `request`: 压缩请求
    ///
    /// 返回:
    /// - 压缩摘要正文
    pub(super) async fn create_compaction_summary(
        &self,
        request: &CompactionRequest,
    ) -> Result<String> {
        let prompt = self
            .state
            .build_compaction_summary_prompt(request, self.context_tokens)?;
        let messages = vec![
            ChatMessage::system(
                "You are a conversation compaction worker. Produce a concise, faithful Markdown summary for future turns. Do not answer the user task.",
            ),
            ChatMessage::plain("user", prompt),
        ];
        let max_chars = crate::state::summary_char_limit(self.context_tokens);
        let first = self.request_compaction_summary(messages).await?;
        if crate::state::validate_summary(&first, max_chars).is_ok() {
            return Ok(first);
        }
        let repair = vec![
            ChatMessage::system(
                "Repair the candidate conversation summary. Return only the required Markdown structure, preserve factual content, and do not answer the user task.",
            ),
            ChatMessage::plain(
                "user",
                format!(
                    "The candidate below failed structural validation. Rewrite it with every required heading in the exact order from the original compaction instructions and keep it under {max_chars} characters.\n\n<candidate-summary>\n{first}\n</candidate-summary>"
                ),
            ),
        ];
        let repaired = self.request_compaction_summary(repair).await?;
        crate::state::validate_summary(&repaired, max_chars)?;
        Ok(repaired)
    }

    /// 请求模型生成一次压缩摘要并累计辅助用量。
    async fn request_compaction_summary(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let _http_debug_session = crate::llm::HttpDebugSessionGuard::new(self.state.session_id());
        let result = self
            .client
            .chat_stream_events(messages, Vec::new(), |_| Ok(()))
            .await?;
        if let Some(usage) = &result.usage {
            self.state.add_auxiliary_usage(usage)?;
        }
        Ok(result.content.trim().to_string())
    }
}
