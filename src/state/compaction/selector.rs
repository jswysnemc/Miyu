use super::model::CompactionRequest;
use super::{estimate_turn_context_chars, should_compact_for_context_chars};
use crate::state::turns::{Turn, TurnStatus};

/// 当前请求上下文已经达到阈值后选择旧轮次进行压缩。
///
/// 参数:
/// - `turns`: 当前全部轮次
/// - `previous_summary`: 已有压缩摘要
/// - `current_context_chars`: 当前请求上下文估算字符数
/// - `context_limit_chars`: 当前模型上下文窗口字符数
/// - `trim_at_ratio`: 自动压缩触发比例
/// - `preserve_recent_chars`: 最近上下文保留预算
///
/// 返回:
/// - 需要执行的压缩请求
pub fn select_compaction_after_context_trigger(
    turns: &[Turn],
    previous_summary: Option<String>,
    current_context_chars: usize,
    context_limit_chars: usize,
    trim_at_ratio: f32,
    preserve_recent_chars: usize,
) -> Option<CompactionRequest> {
    if !should_compact_for_context_chars(current_context_chars, context_limit_chars, trim_at_ratio)
    {
        return None;
    }
    let non_running_indexes = turns
        .iter()
        .enumerate()
        .filter_map(|(index, turn)| (turn.status != TurnStatus::Running).then_some(index))
        .collect::<Vec<_>>();
    if non_running_indexes.is_empty() {
        return None;
    }
    if non_running_indexes.len() <= 2 {
        return None;
    }

    let tail_start = recent_tail_start(turns, &non_running_indexes, preserve_recent_chars);
    let mut selected = non_running_indexes
        .iter()
        .filter(|index| tail_start.map(|start| **index < start).unwrap_or(true))
        .map(|index| turns[*index].clone())
        .collect::<Vec<_>>();

    // 1. 已触发压缩时至少收敛两个旧轮次，避免极小 tail 让压缩无法释放有效空间
    if selected.len() < 2 {
        selected = non_running_indexes
            .iter()
            .take(2)
            .map(|index| turns[*index].clone())
            .collect();
    }
    Some(CompactionRequest::new(selected, previous_summary))
}

/// 按最近上下文预算返回 tail 起始轮次下标。
///
/// 参数:
/// - `turns`: 当前全部轮次
/// - `non_running_indexes`: 可参与选择的非运行轮次下标
/// - `preserve_recent_chars`: 最近上下文保留预算
///
/// 返回:
/// - 被保留 tail 的起始轮次下标
fn recent_tail_start(
    turns: &[Turn],
    non_running_indexes: &[usize],
    preserve_recent_chars: usize,
) -> Option<usize> {
    if preserve_recent_chars == 0 {
        return None;
    }
    let mut total = 0usize;
    let mut keep_start = None;
    for index in non_running_indexes.iter().rev() {
        let size = estimate_turn_context_chars(&turns[*index]);
        if total.saturating_add(size) <= preserve_recent_chars || keep_start.is_none() {
            total = total.saturating_add(size);
            keep_start = Some(*index);
            continue;
        }
        break;
    }
    keep_start
}

#[cfg(test)]
mod tests {
    use super::*;

    fn completed_turn(seq: i64, chars: usize) -> Turn {
        Turn {
            turn_id: format!("turn_{seq}"),
            seq,
            user_content: "u".repeat(chars),
            user_timestamp: "2026-01-01T00:00:00Z".to_string(),
            assistant_content: "a".repeat(chars),
            assistant_reasoning: None,
            assistant_timestamp: Some("2026-01-01T00:00:01Z".to_string()),
            status: TurnStatus::Completed,
            tool_reports: Vec::new(),
        }
    }

    #[test]
    fn preserves_recent_tail_turns() {
        let turns = vec![
            completed_turn(1, 200),
            completed_turn(2, 200),
            completed_turn(3, 200),
            completed_turn(4, 200),
        ];

        let request = select_compaction_after_context_trigger(
            &turns,
            Some("summary".to_string()),
            1_000,
            1_000,
            0.5,
            1_000,
        )
        .expect("compaction request");

        assert_eq!(
            request.compact_turn_ids,
            vec!["turn_1".to_string(), "turn_2".to_string()]
        );
        assert_eq!(request.previous_summary.as_deref(), Some("summary"));
    }

    #[test]
    fn does_not_compact_when_only_tail_exists() {
        let turns = vec![completed_turn(1, 400), completed_turn(2, 400)];

        assert!(
            select_compaction_after_context_trigger(&turns, None, 100, 1_000, 0.5, 1_000).is_none()
        );
    }

    #[test]
    fn context_trigger_can_compact_without_tail_when_budget_is_empty() {
        let turns = vec![
            completed_turn(1, 10),
            completed_turn(2, 10),
            completed_turn(3, 10),
        ];

        let request = select_compaction_after_context_trigger(&turns, None, 900, 1_000, 0.5, 0)
            .expect("compaction request");

        assert_eq!(
            request.compact_turn_ids,
            vec![
                "turn_1".to_string(),
                "turn_2".to_string(),
                "turn_3".to_string()
            ]
        );
    }

    #[test]
    fn preserves_every_recent_turn_that_fits_the_budget() {
        let turns = vec![
            completed_turn(1, 100),
            completed_turn(2, 100),
            completed_turn(3, 100),
            completed_turn(4, 100),
            completed_turn(5, 100),
        ];

        let request = select_compaction_after_context_trigger(&turns, None, 1_000, 1_000, 0.5, 600)
            .expect("compaction request");

        assert_eq!(
            request.compact_turn_ids,
            vec!["turn_1".to_string(), "turn_2".to_string()]
        );
    }
}
