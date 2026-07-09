use super::Agent;
use crate::llm::ChatMessage;
use crate::state::{CompactionApplyOutcome, CompactionRequest};
use anyhow::{bail, Context, Result};

impl Agent {
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
        let result = self
            .client
            .chat_stream_events(messages, Vec::new(), |_| Ok(()))
            .await?;
        let summary = result.content.trim();
        if summary.is_empty() {
            bail!("compaction summary is empty");
        }
        if let Some(usage) = &result.usage {
            self.state.add_auxiliary_usage(usage)?;
        }
        Ok(summary.to_string())
    }
}
