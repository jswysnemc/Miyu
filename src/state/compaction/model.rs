use super::prompt::build_summary_prompt;
use crate::state::turns::Turn;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CompactionRequest {
    pub compact_turn_ids: Vec<String>,
    pub compact_turns: Vec<Turn>,
    pub previous_summary: Option<String>,
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
        }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionSummary {
    pub updated_at: String,
    pub compacted_turns: usize,
    pub summary: String,
}
