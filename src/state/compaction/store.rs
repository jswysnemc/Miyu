use super::{CompactionRequest, CompactionSummary};
use crate::state::StateStore;
use anyhow::Result;

impl StateStore {
    /// 按最近一次 provider usage 选择需要自动压缩的旧会话轮次。
    ///
    /// 参数:
    /// - `context_tokens`: 当前模型上下文窗口 token 数
    /// - `trim_at_ratio`: 触发压缩比例
    ///
    /// 返回:
    /// - 压缩请求，provider usage 未超过阈值时返回空
    pub fn select_compaction(
        &self,
        context_tokens: usize,
        trim_at_ratio: f32,
    ) -> Result<Option<CompactionRequest>> {
        let Some(usage) = self.last_usage()? else {
            return Ok(None);
        };
        if !super::should_compact_for_usage(&usage, context_tokens, trim_at_ratio) {
            return Ok(None);
        }
        let turns = self.conv_db.load_turns()?;
        let previous_summary = self
            .load_compaction_summary()?
            .map(|summary| summary.summary);
        Ok(super::select_compaction_after_token_trigger(
            &turns,
            previous_summary,
        ))
    }

    /// 选择手动压缩的旧会话轮次。
    ///
    /// 参数:
    /// - `keep_tail_turns`: 保留的最近非运行轮次数量
    ///
    /// 返回:
    /// - 压缩请求，没有可压缩旧轮次时返回空
    pub fn select_manual_compaction(
        &self,
        keep_tail_turns: usize,
    ) -> Result<Option<CompactionRequest>> {
        let turns = self.conv_db.load_turns()?;
        let previous_summary = self
            .load_compaction_summary()?
            .map(|summary| summary.summary);
        Ok(super::select_manual_compaction(
            &turns,
            previous_summary,
            keep_tail_turns,
        ))
    }

    /// 应用自动压缩结果。
    ///
    /// 参数:
    /// - `request`: 压缩请求
    /// - `summary`: 模型生成的摘要正文
    ///
    /// 返回:
    /// - 应用是否成功
    pub fn apply_compaction(&self, request: &CompactionRequest, summary: &str) -> Result<()> {
        let previous_count = self
            .load_compaction_summary()?
            .map(|summary| summary.compacted_turns)
            .unwrap_or_default();
        super::save_summary(
            &self.compaction_summary_file(),
            summary,
            previous_count + request.turn_count(),
        )?;
        self.conv_db
            .delete_turns_by_ids(&request.compact_turn_ids)?;
        self.clear_last_usage()?;
        Ok(())
    }

    /// 读取可注入上下文的压缩摘要消息。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 压缩摘要上下文消息
    pub fn compaction_summary_context(&self) -> Result<Option<String>> {
        Ok(self
            .load_compaction_summary()?
            .map(|summary| super::summary_context_message(&summary.summary)))
    }

    /// 读取压缩摘要。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 压缩摘要
    fn load_compaction_summary(&self) -> Result<Option<CompactionSummary>> {
        super::load_summary(&self.compaction_summary_file())
    }

    /// 清理压缩摘要。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 清理是否成功
    pub fn clear_compaction_summary(&self) -> Result<()> {
        super::clear_summary(&self.compaction_summary_file())
    }
}
