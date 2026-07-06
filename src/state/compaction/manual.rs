use super::model::CompactionRequest;
use crate::state::turns::{Turn, TurnStatus};

/// 选择手动压缩的旧轮次。
///
/// 参数:
/// - `turns`: 当前全部轮次
/// - `previous_summary`: 已有压缩摘要
/// - `keep_tail_turns`: 保留的最近非运行轮次数量
///
/// 返回:
/// - 需要执行的压缩请求
pub fn select_manual_compaction(
    turns: &[Turn],
    previous_summary: Option<String>,
    keep_tail_turns: usize,
) -> Option<CompactionRequest> {
    let non_running_indexes = turns
        .iter()
        .enumerate()
        .filter_map(|(index, turn)| (turn.status != TurnStatus::Running).then_some(index))
        .collect::<Vec<_>>();
    if non_running_indexes.len() <= keep_tail_turns {
        return None;
    }

    let selected_len = non_running_indexes.len().saturating_sub(keep_tail_turns);
    let selected = non_running_indexes
        .into_iter()
        .take(selected_len)
        .map(|index| turns[index].clone())
        .collect::<Vec<_>>();
    if selected.is_empty() {
        None
    } else {
        Some(CompactionRequest::new(selected, previous_summary))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn completed_turn(seq: i64) -> Turn {
        Turn {
            turn_id: format!("turn_{seq}"),
            seq,
            user_content: "u".to_string(),
            user_timestamp: "2026-01-01T00:00:00Z".to_string(),
            assistant_content: "a".to_string(),
            assistant_reasoning: None,
            assistant_timestamp: Some("2026-01-01T00:00:01Z".to_string()),
            status: TurnStatus::Completed,
            tool_reports: Vec::new(),
        }
    }

    #[test]
    fn keeps_requested_tail_turns() {
        let turns = vec![completed_turn(1), completed_turn(2), completed_turn(3)];
        let request = select_manual_compaction(&turns, None, 1).expect("compaction request");

        assert_eq!(
            request.compact_turn_ids,
            vec!["turn_1".to_string(), "turn_2".to_string()]
        );
    }

    #[test]
    fn skips_when_tail_covers_all_turns() {
        let turns = vec![completed_turn(1), completed_turn(2)];

        assert!(select_manual_compaction(&turns, None, 2).is_none());
    }
}
