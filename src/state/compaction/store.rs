use super::{CompactionRequest, CompactionSummary};
use crate::llm::ChatMessage;
use crate::state::request_projection::{
    estimate_projected_request_chars, project_provider_turn_from_messages, ProjectedRequest,
};
use crate::state::session_memory::compact::{self, MemoryCompactDecision};
use crate::state::tool_history::build_budgeted_summary_history;
use crate::state::StateStore;
use anyhow::{bail, Result};

const SUMMARY_PROMPT_HISTORY_RESERVE_CHARS: usize = 512;

/// 压缩应用结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionApplyOutcome {
    Applied,
    RejectedOverBudget,
}

/// 压缩写入前的预算预检结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionBudgetCheck {
    pub context_chars: usize,
    pub context_limit_chars: usize,
    pub result_chars: usize,
}

impl CompactionBudgetCheck {
    /// 判断压缩后重新投影是否超过预算。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 是否超过上下文预算
    pub fn is_over_budget(&self) -> bool {
        self.result_chars > self.context_limit_chars
    }
}

impl StateStore {
    /// 按当前请求上下文估算选择需要自动压缩的旧会话轮次。
    ///
    /// 参数:
    /// - `messages`: 当前请求消息列表
    /// - `context_limit_chars`: 当前模型上下文窗口字符数
    /// - `trim_at_ratio`: 触发压缩比例
    /// - `trim_batch_ratio`: 最近上下文保留比例
    ///
    /// 返回:
    /// - 压缩请求，当前上下文未超过阈值时返回空
    #[allow(dead_code)]
    pub fn select_compaction_for_messages(
        &self,
        messages: &[ChatMessage],
        context_limit_chars: usize,
        trim_at_ratio: f32,
        trim_batch_ratio: f32,
    ) -> Result<Option<CompactionRequest>> {
        let projection = project_provider_turn_from_messages(messages, 0, context_limit_chars);
        self.select_compaction_for_projection(&projection, trim_at_ratio, trim_batch_ratio)
    }

    /// 按当前请求上下文估算选择传统 checkpoint compact 轮次。
    ///
    /// 参数:
    /// - `messages`: 当前请求消息列表
    /// - `context_limit_chars`: 当前模型上下文窗口字符数
    /// - `trim_at_ratio`: 触发压缩比例
    /// - `trim_batch_ratio`: 最近上下文保留比例
    ///
    /// 返回:
    /// - 压缩请求，当前上下文未超过阈值时返回空
    #[allow(dead_code)]
    pub fn select_legacy_compaction_for_messages(
        &self,
        messages: &[ChatMessage],
        context_limit_chars: usize,
        trim_at_ratio: f32,
        trim_batch_ratio: f32,
    ) -> Result<Option<CompactionRequest>> {
        let projection = project_provider_turn_from_messages(messages, 0, context_limit_chars);
        self.select_legacy_compaction_for_projection(&projection, trim_at_ratio, trim_batch_ratio)
    }

    /// 按 provider 请求投影视图选择需要自动压缩的旧会话轮次。
    ///
    /// 参数:
    /// - `projection`: 当前 provider 请求投影视图
    /// - `trim_at_ratio`: 触发压缩比例
    /// - `trim_batch_ratio`: 最近上下文保留比例
    ///
    /// 返回:
    /// - 压缩请求，当前上下文未超过阈值时返回空
    pub fn select_compaction_for_projection(
        &self,
        projection: &ProjectedRequest,
        trim_at_ratio: f32,
        trim_batch_ratio: f32,
    ) -> Result<Option<CompactionRequest>> {
        let current_context_chars = estimate_projected_request_chars(projection);
        let context_limit_chars = projection.estimate.context_limit_chars;
        if !super::should_compact_for_context_chars(
            current_context_chars,
            context_limit_chars,
            trim_at_ratio,
        ) {
            return Ok(None);
        }
        let preserve_recent_chars =
            ((context_limit_chars as f32) * trim_batch_ratio).max(1.0) as usize;
        let turns = self.conv_db.load_turns()?;
        let memory =
            crate::state::session_memory::repository::load_memory(&self.conv_db, &self.session_id)?;
        let latest_checkpoint = {
            let conn = self.conv_db.conn.lock().unwrap();
            crate::state::checkpoints::load_latest_checkpoint(&conn)?
        };
        match compact::build_memory_compaction_input(
            memory.as_ref(),
            latest_checkpoint.as_ref(),
            &turns,
        ) {
            MemoryCompactDecision::MemoryInput(input) => {
                return Ok(Some(input.request));
            }
            MemoryCompactDecision::Fallback { reason } => {
                compact::record_fallback_recovery(
                    &self.conv_db,
                    &self.session_id,
                    reason,
                    current_context_chars,
                    context_limit_chars,
                )?;
            }
        }
        let previous_summary = self
            .load_compaction_summary()?
            .map(|summary| summary.summary);
        self.select_legacy_compaction_from_parts(
            &turns,
            previous_summary,
            current_context_chars,
            context_limit_chars,
            trim_at_ratio,
            preserve_recent_chars,
        )
    }

    /// 按 provider 请求投影视图选择传统 checkpoint compact 轮次。
    ///
    /// 参数:
    /// - `projection`: 当前 provider 请求投影视图
    /// - `trim_at_ratio`: 触发压缩比例
    /// - `trim_batch_ratio`: 最近上下文保留比例
    ///
    /// 返回:
    /// - 压缩请求，当前上下文未超过阈值时返回空
    pub fn select_legacy_compaction_for_projection(
        &self,
        projection: &ProjectedRequest,
        trim_at_ratio: f32,
        trim_batch_ratio: f32,
    ) -> Result<Option<CompactionRequest>> {
        let current_context_chars = estimate_projected_request_chars(projection);
        let context_limit_chars = projection.estimate.context_limit_chars;
        if !super::should_compact_for_context_chars(
            current_context_chars,
            context_limit_chars,
            trim_at_ratio,
        ) {
            return Ok(None);
        }
        let preserve_recent_chars =
            ((context_limit_chars as f32) * trim_batch_ratio).max(1.0) as usize;
        let turns = self.conv_db.load_turns()?;
        let previous_summary = self
            .load_compaction_summary()?
            .map(|summary| summary.summary);
        self.select_legacy_compaction_from_parts(
            &turns,
            previous_summary,
            current_context_chars,
            context_limit_chars,
            trim_at_ratio,
            preserve_recent_chars,
        )
    }

    /// 从已读取状态选择传统 checkpoint compact 轮次。
    ///
    /// 参数:
    /// - `turns`: 当前会话轮次
    /// - `previous_summary`: 既有压缩摘要
    /// - `current_context_chars`: 当前上下文字符估算
    /// - `context_limit_chars`: 上下文预算字符数
    /// - `trim_at_ratio`: 触发压缩比例
    /// - `preserve_recent_chars`: 最近上下文保留预算
    ///
    /// 返回:
    /// - 压缩请求
    fn select_legacy_compaction_from_parts(
        &self,
        turns: &[crate::state::Turn],
        previous_summary: Option<String>,
        current_context_chars: usize,
        context_limit_chars: usize,
        trim_at_ratio: f32,
        preserve_recent_chars: usize,
    ) -> Result<Option<CompactionRequest>> {
        Ok(super::select_compaction_after_context_trigger(
            turns,
            previous_summary,
            current_context_chars,
            context_limit_chars,
            trim_at_ratio,
            preserve_recent_chars,
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

    /// 构造带工具历史预算的压缩摘要提示词。
    ///
    /// 参数:
    /// - `request`: 压缩请求
    /// - `context_limit_chars`: 当前模型上下文窗口字符数
    ///
    /// 返回:
    /// - 可发送给压缩模型的提示词
    pub(crate) fn build_compaction_summary_prompt(
        &self,
        request: &CompactionRequest,
        context_limit_chars: usize,
    ) -> Result<String> {
        let overhead = super::prompt::build_summary_prompt_from_history(
            request.previous_summary.as_deref(),
            "",
        )
        .chars()
        .count();
        let history_budget = context_limit_chars
            .saturating_sub(overhead)
            .saturating_sub(SUMMARY_PROMPT_HISTORY_RESERVE_CHARS);
        let history = build_budgeted_summary_history(
            &self.conv_db,
            &self.session_id,
            Some(&self.state_dir),
            &request.compact_turns,
            history_budget,
        )?;
        let prompt = super::prompt::build_summary_prompt_from_history(
            request.previous_summary.as_deref(),
            &history.history,
        );
        let prompt_chars = prompt.chars().count();
        if history.replacement_missing_count > 0 {
            self.record_recovery_failure(
                request.compact_turn_ids.last().map(String::as_str),
                crate::state::FailureKind::ToolHistoryReplacementMissing,
                crate::state::RecoveryStatus::Observed,
                &format!(
                    "压缩摘要输入发现 {} 个工具输出引用缺少稳定 replacement，已回退使用 result_preview",
                    history.replacement_missing_count
                ),
                0,
                prompt_chars,
                context_limit_chars,
            )?;
        }
        if history.result_ref_missing_file_count > 0 {
            self.record_recovery_failure(
                request.compact_turn_ids.last().map(String::as_str),
                crate::state::FailureKind::ToolHistoryReplacementMissing,
                crate::state::RecoveryStatus::Observed,
                &format!(
                    "压缩摘要输入发现 {} 个工具完整输出引用文件缺失，已回退使用 result_preview",
                    history.result_ref_missing_file_count
                ),
                0,
                prompt_chars,
                context_limit_chars,
            )?;
        }
        if prompt_chars > context_limit_chars
            || (history.history.is_empty() && request.turn_count() > 0)
        {
            bail!(
                "tool history summary prompt over budget: prompt_chars={prompt_chars}, context_limit_chars={context_limit_chars}, history_budget_chars={history_budget}"
            );
        }
        Ok(prompt)
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
        let source_turn_count = request.source_turn_count_after_compaction(previous_count);
        crate::state::checkpoints::apply_checkpoint_compaction(
            &self.conv_db,
            request,
            summary,
            source_turn_count,
            crate::state::checkpoints::CheckpointReason::Auto,
        )?;
        super::save_summary(&self.compaction_summary_file(), summary, source_turn_count)?;
        self.resolve_active_compaction_failures()?;
        Ok(())
    }

    /// 在预算内应用压缩结果。
    ///
    /// 参数:
    /// - `request`: 压缩请求
    /// - `summary`: 模型生成的摘要正文
    /// - `context_chars`: 当前上下文字符估算
    /// - `context_limit_chars`: 上下文预算字符数
    ///
    /// 返回:
    /// - 压缩应用结果
    pub fn apply_compaction_with_budget_guard(
        &self,
        request: &CompactionRequest,
        summary: &str,
        projection: &ProjectedRequest,
        exclude_turn_id: Option<&str>,
    ) -> Result<CompactionApplyOutcome> {
        let budget = self.compaction_budget_check(request, summary, projection, exclude_turn_id)?;
        if budget.is_over_budget() {
            self.record_auto_compaction_failure(
                request.compact_turn_ids.last().map(String::as_str),
                crate::state::FailureKind::CompactionOverBudget,
                &format!(
                    "compaction reprojected provider request over budget: result_chars={}, context_limit_chars={}",
                    budget.result_chars, budget.context_limit_chars
                ),
                budget.context_chars,
                budget.context_limit_chars,
            )?;
            return Ok(CompactionApplyOutcome::RejectedOverBudget);
        }
        self.apply_compaction(request, summary)?;
        Ok(CompactionApplyOutcome::Applied)
    }

    /// 在预算内应用手动压缩结果。
    ///
    /// 参数:
    /// - `request`: 压缩请求
    /// - `summary`: 模型生成的摘要正文
    /// - `context_limit_chars`: 上下文预算字符数
    ///
    /// 返回:
    /// - 压缩应用结果
    pub fn apply_manual_compaction_with_budget_guard(
        &self,
        request: &CompactionRequest,
        summary: &str,
        context_limit_chars: usize,
    ) -> Result<CompactionApplyOutcome> {
        let budget = self.manual_compaction_budget_check(request, summary, context_limit_chars)?;
        if budget.is_over_budget() {
            self.record_manual_compaction_failure(
                crate::state::FailureKind::CompactionOverBudget,
                &format!(
                    "manual compaction reprojected history over budget: result_chars={}, context_limit_chars={}",
                    budget.result_chars, budget.context_limit_chars
                ),
                budget.context_chars,
                budget.context_limit_chars,
            )?;
            return Ok(CompactionApplyOutcome::RejectedOverBudget);
        }
        self.apply_compaction(request, summary)?;
        Ok(CompactionApplyOutcome::Applied)
    }

    /// 预检压缩写入后的 provider 请求预算。
    ///
    /// 参数:
    /// - `request`: 压缩请求
    /// - `summary`: 模型生成的摘要正文
    /// - `projection`: 当前 provider 请求投影视图
    /// - `exclude_turn_id`: 当前运行中轮次标识
    ///
    /// 返回:
    /// - 预算预检结果
    pub fn compaction_budget_check(
        &self,
        request: &CompactionRequest,
        summary: &str,
        projection: &ProjectedRequest,
        exclude_turn_id: Option<&str>,
    ) -> Result<CompactionBudgetCheck> {
        let result_chars = self.estimate_reprojected_context_chars_after_compaction(
            request,
            summary,
            projection,
            exclude_turn_id,
        )?;
        Ok(CompactionBudgetCheck {
            context_chars: projection.estimate.message_chars,
            context_limit_chars: projection.estimate.context_limit_chars,
            result_chars,
        })
    }

    /// 预检手动压缩写入后的历史预算。
    ///
    /// 参数:
    /// - `request`: 压缩请求
    /// - `summary`: 模型生成的摘要正文
    /// - `context_limit_chars`: 上下文预算字符数
    ///
    /// 返回:
    /// - 预算预检结果
    pub fn manual_compaction_budget_check(
        &self,
        request: &CompactionRequest,
        summary: &str,
        context_limit_chars: usize,
    ) -> Result<CompactionBudgetCheck> {
        let context_chars = self.visible_history_context_chars(None)?;
        let result_chars = self.projected_history_chars_after_compaction(request, summary, None)?;
        Ok(CompactionBudgetCheck {
            context_chars,
            context_limit_chars,
            result_chars,
        })
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
    pub(crate) fn load_compaction_summary(&self) -> Result<Option<CompactionSummary>> {
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
