use super::prompt::build_summary_prompt;
use crate::state::turns::Turn;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CompactionRequest {
    pub compact_turn_ids: Vec<String>,
    pub compact_turns: Vec<Turn>,
    pub previous_summary: Option<String>,
    coverage: Option<CompactionCoverage>,
}

impl CompactionRequest {
    /// 创建会话压缩请求。
    ///
    /// 参数:
    /// - `compact_turns`: 需要压缩的旧轮次
    /// - `previous_summary`: 上一次压缩摘要
    ///
    /// 返回:
    /// - 会话压缩请求
    pub fn new(compact_turns: Vec<Turn>, previous_summary: Option<String>) -> Self {
        let compact_turn_ids = compact_turns
            .iter()
            .map(|turn| turn.turn_id.clone())
            .collect();
        Self {
            compact_turn_ids,
            compact_turns,
            previous_summary,
            coverage: None,
        }
    }

    /// 附加 memory-first compact 的覆盖范围。
    ///
    /// 参数:
    /// - `compacted_from_seq`: checkpoint 覆盖起始 seq
    /// - `compacted_to_seq`: checkpoint 覆盖结束 seq
    /// - `delete_through_seq`: 需要删除的最大原始轮次 seq
    /// - `source_turn_count`: checkpoint 累计覆盖轮次数
    ///
    /// 返回:
    /// - 带覆盖范围的会话压缩请求
    pub(crate) fn with_coverage(
        mut self,
        compacted_from_seq: i64,
        compacted_to_seq: i64,
        delete_through_seq: i64,
        source_turn_count: usize,
    ) -> Self {
        self.coverage = Some(CompactionCoverage {
            compacted_from_seq,
            compacted_to_seq,
            delete_through_seq,
            source_turn_count,
        });
        self
    }

    /// 返回需要压缩的轮次数量。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 轮次数量
    pub fn turn_count(&self) -> usize {
        self.compact_turns.len()
    }

    /// 返回被压缩轮次 seq 范围。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 起止 seq，没有待压缩轮次时返回空
    pub(crate) fn seq_range(&self) -> Option<(i64, i64)> {
        if let Some(coverage) = &self.coverage {
            return Some((coverage.compacted_from_seq, coverage.compacted_to_seq));
        }
        let first = self.compact_turns.first()?;
        let last = self.compact_turns.last()?;
        Some((first.seq, last.seq))
    }

    /// 返回覆盖来源轮次数。
    ///
    /// 参数:
    /// - `previous_count`: 既有 checkpoint 覆盖轮次数
    ///
    /// 返回:
    /// - 新 checkpoint 应记录的累计覆盖轮次数
    pub(crate) fn source_turn_count_after_compaction(&self, previous_count: usize) -> usize {
        self.coverage
            .as_ref()
            .map(|coverage| coverage.source_turn_count)
            .unwrap_or_else(|| previous_count + self.turn_count())
    }

    /// 返回 memory-first compact 需要删除的最大原始轮次 seq。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 最大删除 seq；普通 compact 返回空
    pub(crate) fn delete_through_seq(&self) -> Option<i64> {
        self.coverage
            .as_ref()
            .map(|coverage| coverage.delete_through_seq)
    }

    /// 判断请求是否来自 memory-first compact。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 是否携带 Session Memory 覆盖范围
    pub(crate) fn is_memory_first(&self) -> bool {
        self.coverage.is_some()
    }

    /// 构造 checkpoint recent 上下文。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 最近被压缩轮次的可读文本
    pub(crate) fn recent_context(&self) -> String {
        self.compact_turns
            .iter()
            .rev()
            .take(2)
            .rev()
            .map(|turn| {
                format!(
                    "User: {}\nAssistant: {}",
                    turn.user_content, turn.assistant_content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// 构造压缩摘要提示词。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 摘要提示词
    pub fn prompt(&self) -> String {
        build_summary_prompt(self.previous_summary.as_deref(), &self.compact_turns)
    }
}

/// 压缩覆盖范围。
#[derive(Debug, Clone)]
struct CompactionCoverage {
    compacted_from_seq: i64,
    compacted_to_seq: i64,
    delete_through_seq: i64,
    source_turn_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionSummary {
    pub updated_at: String,
    pub compacted_turns: usize,
    pub summary: String,
}
