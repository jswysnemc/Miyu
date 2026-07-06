use super::model::CompactionRequest;
use crate::state::turns::{turn_chars, Turn, TurnStatus};

const DEFAULT_TAIL_TURNS: usize = 2;

/// 选择需要压缩的会话轮次。
///
/// 参数:
/// - `turns`: 当前全部轮次
/// - `previous_summary`: 已有压缩摘要
/// - `max_chars`: 最大上下文字符数
/// - `trim_at_ratio`: 触发压缩比例
/// - `trim_batch_ratio`: 压缩目标批量比例
///
/// 返回:
/// - 需要执行的压缩请求
pub fn select_compaction(
    turns: &[Turn],
    previous_summary: Option<String>,
    max_chars: usize,
    trim_at_ratio: f32,
    trim_batch_ratio: f32,
) -> Option<CompactionRequest> {
    if max_chars == 0 || turns.is_empty() {
        return None;
    }
    let trigger = (max_chars as f32 * trim_at_ratio).max(1.0) as usize;
    let mut total = turns.iter().map(turn_chars).sum::<usize>();
    if total <= trigger {
        return None;
    }

    let target = max_chars.saturating_sub((max_chars as f32 * trim_batch_ratio).max(1.0) as usize);
    let non_running_indexes = turns
        .iter()
        .enumerate()
        .filter_map(|(index, turn)| (turn.status != TurnStatus::Running).then_some(index))
        .collect::<Vec<_>>();
    if non_running_indexes.len() <= DEFAULT_TAIL_TURNS {
        return None;
    }

    let selectable_len = non_running_indexes.len().saturating_sub(DEFAULT_TAIL_TURNS);
    let mut selected = Vec::new();
    for index in non_running_indexes.into_iter().take(selectable_len) {
        if total <= target {
            break;
        }
        let turn = turns[index].clone();
        total = total.saturating_sub(turn_chars(&turn));
        selected.push(turn);
    }

    if selected.is_empty() {
        None
    } else {
        Some(CompactionRequest::new(selected, previous_summary))
    }
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
    fn skips_when_under_trigger() {
        let turns = vec![completed_turn(1, 10), completed_turn(2, 10)];

        assert!(select_compaction(&turns, None, 1000, 0.8, 0.2).is_none());
    }

    #[test]
    fn preserves_recent_tail_turns() {
        let turns = vec![
            completed_turn(1, 200),
            completed_turn(2, 200),
            completed_turn(3, 200),
            completed_turn(4, 200),
        ];

        let request = select_compaction(&turns, Some("summary".to_string()), 1000, 0.5, 0.2)
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

        assert!(select_compaction(&turns, None, 1000, 0.5, 0.2).is_none());
    }
}
