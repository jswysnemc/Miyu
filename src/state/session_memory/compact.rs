use super::model::SessionMemory;
use crate::state::checkpoints::CompactionCheckpoint;
use crate::state::failure_recovery::{self, FailureKind, NewRecoveryRecord, RecoveryStatus};
use crate::state::turns::ConversationDb;
use crate::state::turns::{Turn, TurnStatus};
use crate::state::CompactionRequest;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Session Memory 压缩输入。
#[derive(Debug, Clone)]
pub(crate) struct MemoryCompactInput {
    pub request: CompactionRequest,
    #[allow(dead_code)]
    pub memory_boundary_seq: i64,
}

/// Session Memory 压缩回退原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemoryCompactFallbackReason {
    MissingMemory,
    EmptySummary,
    Disabled,
    BoundaryBeforeCheckpoint,
    BoundaryAfterLatestTurn,
    CheckpointMismatch,
    EmptyTail,
}

impl MemoryCompactFallbackReason {
    /// 返回稳定恢复原因文本。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 恢复原因文本
    fn as_str(self) -> &'static str {
        match self {
            Self::MissingMemory => "missing_memory",
            Self::EmptySummary => "empty_summary",
            Self::Disabled => "disabled",
            Self::BoundaryBeforeCheckpoint => "boundary_before_checkpoint",
            Self::BoundaryAfterLatestTurn => "boundary_after_latest_turn",
            Self::CheckpointMismatch => "checkpoint_mismatch",
            Self::EmptyTail => "empty_tail",
        }
    }

    /// 判断是否应写入 boundary invalid recovery。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 是否需要记录恢复事实
    fn should_record_boundary_invalid(self) -> bool {
        matches!(
            self,
            Self::BoundaryBeforeCheckpoint
                | Self::BoundaryAfterLatestTurn
                | Self::CheckpointMismatch
        )
    }
}

/// Session Memory 压缩输入构造结果。
#[derive(Debug, Clone)]
pub(crate) enum MemoryCompactDecision {
    MemoryInput(MemoryCompactInput),
    Fallback { reason: MemoryCompactFallbackReason },
}

/// 记录 Session Memory compact fallback 恢复事实。
///
/// 参数:
/// - `db`: 对话数据库
/// - `session_id`: 当前会话标识
/// - `reason`: fallback 原因
/// - `context_chars`: 当前上下文字符数
/// - `context_limit_chars`: 当前上下文预算字符数
///
/// 返回:
/// - 写入是否成功
pub(crate) fn record_fallback_recovery(
    db: &ConversationDb,
    session_id: &str,
    reason: MemoryCompactFallbackReason,
    context_chars: usize,
    context_limit_chars: usize,
) -> Result<()> {
    if !reason.should_record_boundary_invalid() {
        return Ok(());
    }
    failure_recovery::record_failure(
        db,
        NewRecoveryRecord {
            session_id: session_id.to_string(),
            turn_id: None,
            kind: FailureKind::SessionMemoryBoundaryInvalid,
            status: RecoveryStatus::Observed,
            reason: format!("session memory compact fallback: {}", reason.as_str()),
            retry_count: 0,
            checkpoint_id: failure_recovery::latest_checkpoint_id(db)?,
            context_chars,
            context_limit_chars,
        },
    )?;
    Ok(())
}

impl MemoryCompactDecision {
    /// 测试用：返回 Session Memory 压缩输入。
    ///
    /// 返回:
    /// - Session Memory 压缩输入
    #[cfg(test)]
    fn expect_memory_input(self) -> MemoryCompactInput {
        match self {
            Self::MemoryInput(input) => input,
            Self::Fallback { reason } => panic!("expected memory compact input, got {reason:?}"),
        }
    }

    /// 测试用：返回回退原因。
    ///
    /// 返回:
    /// - 回退原因
    #[cfg(test)]
    fn expect_fallback_reason(self) -> MemoryCompactFallbackReason {
        match self {
            Self::MemoryInput(_) => panic!("expected fallback decision"),
            Self::Fallback { reason } => reason,
        }
    }
}

/// 从 Session Memory 和 tail turns 构造 memory-first compact 输入。
///
/// 参数:
/// - `memory`: 当前会话工作记忆
/// - `latest_checkpoint`: 最近一次权威 checkpoint
/// - `turns`: 当前仍保留的原始轮次
///
/// 返回:
/// - 可用的 memory-first compact 输入，或明确回退原因
pub(crate) fn build_memory_compaction_input(
    memory: Option<&SessionMemory>,
    latest_checkpoint: Option<&CompactionCheckpoint>,
    turns: &[Turn],
) -> MemoryCompactDecision {
    let Some(memory) = memory else {
        return fallback(MemoryCompactFallbackReason::MissingMemory);
    };
    if memory.summary.trim().is_empty() {
        return fallback(MemoryCompactFallbackReason::EmptySummary);
    }
    if is_disabled(memory) {
        return fallback(MemoryCompactFallbackReason::Disabled);
    }
    if !checkpoint_matches(memory, latest_checkpoint) {
        return fallback(MemoryCompactFallbackReason::CheckpointMismatch);
    }

    // 1. 先用最新 checkpoint 校验 memory 不得落后于权威压缩边界
    let checkpoint_seq = latest_checkpoint
        .map(|checkpoint| checkpoint.compacted_to_seq)
        .unwrap_or_default();
    if memory.last_summarized_seq < checkpoint_seq {
        return fallback(MemoryCompactFallbackReason::BoundaryBeforeCheckpoint);
    }

    // 2. 再用 checkpoint 和当前非运行轮次共同确定最新可信 seq
    let latest_turn_seq = turns
        .iter()
        .filter(|turn| turn.status != TurnStatus::Running)
        .map(|turn| turn.seq)
        .max()
        .unwrap_or_default()
        .max(checkpoint_seq);
    if memory.last_summarized_seq > latest_turn_seq {
        return fallback(MemoryCompactFallbackReason::BoundaryAfterLatestTurn);
    }

    // 3. 最后只选择 memory 边界后的非运行 tail，避免压缩运行中轮次
    let tail_turns = turns
        .iter()
        .filter(|turn| turn.status != TurnStatus::Running)
        .filter(|turn| turn.seq > memory.last_summarized_seq)
        .cloned()
        .collect::<Vec<_>>();
    if tail_turns.is_empty() {
        return fallback(MemoryCompactFallbackReason::EmptyTail);
    }

    let compacted_from_seq = latest_checkpoint
        .map(|checkpoint| checkpoint.compacted_from_seq)
        .unwrap_or(1);
    let compacted_to_seq = tail_turns
        .last()
        .map(|turn| turn.seq)
        .unwrap_or(memory.last_summarized_seq);
    let source_turn_count = memory.source_turn_count + tail_turns.len();
    let request = CompactionRequest::new(tail_turns, Some(memory.summary.trim().to_string()))
        .with_coverage(
            compacted_from_seq,
            compacted_to_seq,
            compacted_to_seq,
            source_turn_count,
        );

    MemoryCompactDecision::MemoryInput(MemoryCompactInput {
        request,
        memory_boundary_seq: memory.last_summarized_seq,
    })
}

/// 构造回退结果。
///
/// 参数:
/// - `reason`: 回退原因
///
/// 返回:
/// - 回退决策
fn fallback(reason: MemoryCompactFallbackReason) -> MemoryCompactDecision {
    MemoryCompactDecision::Fallback { reason }
}

/// 判断 Session Memory 是否处于熔断期。
///
/// 参数:
/// - `memory`: 当前会话工作记忆
///
/// 返回:
/// - 是否处于熔断期
fn is_disabled(memory: &SessionMemory) -> bool {
    let Some(disabled_until) = memory.disabled_until.as_deref() else {
        return false;
    };
    DateTime::parse_from_rfc3339(disabled_until)
        .map(|until| until.with_timezone(&Utc) > Utc::now())
        .unwrap_or(true)
}

/// 校验 Session Memory 关联的 checkpoint 是否仍是最新 checkpoint。
///
/// 参数:
/// - `memory`: 当前会话工作记忆
/// - `latest_checkpoint`: 最近一次权威 checkpoint
///
/// 返回:
/// - checkpoint 关联是否有效
fn checkpoint_matches(
    memory: &SessionMemory,
    latest_checkpoint: Option<&CompactionCheckpoint>,
) -> bool {
    match latest_checkpoint {
        Some(checkpoint) => memory.checkpoint_id.as_deref() == Some(checkpoint.id.as_str()),
        None => memory.checkpoint_id.is_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::checkpoints::{CheckpointReason, CompactionCheckpoint};
    use crate::state::session_memory::model::SessionMemory;
    use crate::state::turns::{Turn, TurnStatus};

    fn memory(last_summarized_seq: i64, checkpoint_id: Option<&str>) -> SessionMemory {
        SessionMemory {
            session_id: "default".to_string(),
            summary: "memory summary".to_string(),
            last_summarized_turn_id: Some(format!("turn_{last_summarized_seq}")),
            last_summarized_seq,
            checkpoint_id: checkpoint_id.map(str::to_string),
            source_turn_count: last_summarized_seq as usize,
            token_estimate: 16,
            consecutive_failures: 0,
            disabled_until: None,
            last_error: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn checkpoint(compacted_to_seq: i64) -> CompactionCheckpoint {
        CompactionCheckpoint {
            id: "cp_1".to_string(),
            seq: compacted_to_seq,
            compacted_from_seq: 1,
            compacted_to_seq,
            summary: "checkpoint summary".to_string(),
            recent: "checkpoint recent".to_string(),
            source_turn_count: compacted_to_seq as usize,
            reason: CheckpointReason::Auto,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    fn turn(seq: i64, status: TurnStatus) -> Turn {
        Turn {
            turn_id: format!("turn_{seq}"),
            seq,
            user_content: format!("user {seq}"),
            user_timestamp: "2026-01-01T00:00:00Z".to_string(),
            assistant_content: format!("assistant {seq}"),
            assistant_reasoning: None,
            assistant_timestamp: Some("2026-01-01T00:00:01Z".to_string()),
            status,
            tool_reports: Vec::new(),
        }
    }

    #[test]
    fn memory_compact_uses_memory_plus_tail() {
        let turns = vec![
            turn(1, TurnStatus::Completed),
            turn(2, TurnStatus::Completed),
            turn(3, TurnStatus::Completed),
            turn(4, TurnStatus::Interrupted),
            turn(5, TurnStatus::Running),
        ];

        let decision = build_memory_compaction_input(
            Some(&memory(2, Some("cp_1"))),
            Some(&checkpoint(2)),
            &turns,
        );

        let input = decision.expect_memory_input();
        assert_eq!(input.memory_boundary_seq, 2);
        assert_eq!(
            input.request.previous_summary.as_deref(),
            Some("memory summary")
        );
        assert_eq!(
            input.request.compact_turn_ids,
            vec!["turn_3".to_string(), "turn_4".to_string()]
        );
    }

    #[test]
    fn memory_boundary_before_checkpoint_is_rejected() {
        let turns = vec![
            turn(3, TurnStatus::Completed),
            turn(4, TurnStatus::Completed),
        ];

        let decision = build_memory_compaction_input(
            Some(&memory(2, Some("cp_1"))),
            Some(&checkpoint(3)),
            &turns,
        );

        assert_eq!(
            decision.expect_fallback_reason(),
            MemoryCompactFallbackReason::BoundaryBeforeCheckpoint
        );
    }

    #[test]
    fn memory_boundary_after_latest_turn_is_rejected() {
        let turns = vec![
            turn(1, TurnStatus::Completed),
            turn(2, TurnStatus::Completed),
        ];

        let decision = build_memory_compaction_input(Some(&memory(3, None)), None, &turns);

        assert_eq!(
            decision.expect_fallback_reason(),
            MemoryCompactFallbackReason::BoundaryAfterLatestTurn
        );
    }

    #[test]
    fn checkpoint_mismatch_is_rejected() {
        let turns = vec![
            turn(1, TurnStatus::Completed),
            turn(2, TurnStatus::Completed),
        ];

        let decision = build_memory_compaction_input(
            Some(&memory(1, Some("old_cp"))),
            Some(&checkpoint(1)),
            &turns,
        );

        assert_eq!(
            decision.expect_fallback_reason(),
            MemoryCompactFallbackReason::CheckpointMismatch
        );
    }
}
