use super::model::CompactionRequest;
use crate::state::turns::{Turn, TurnStatus};

const DEFAULT_TAIL_TURNS: usize = 2;

/// provider usage 已经达到 token 阈值后选择旧轮次进行压缩。
///
/// 参数:
/// - `turns`: 当前全部轮次
/// - `previous_summary`: 已有压缩摘要
///
/// 返回:
/// - 需要执行的压缩请求
pub fn select_compaction_after_token_trigger(
    turns: &[Turn],
    previous_summary: Option<String>,
) -> Option<CompactionRequest> {
    if turns.is_empty() {
        return None;
    }
    let non_running_indexes = turns
        .iter()
        .enumerate()
        .filter_map(|(index, turn)| (turn.status != TurnStatus::Running).then_some(index))
        .collect::<Vec<_>>();
    if non_running_indexes.len() <= DEFAULT_TAIL_TURNS {
        return None;
    }

    let selectable_len = non_running_indexes.len().saturating_sub(DEFAULT_TAIL_TURNS);
    let selected = non_running_indexes
        .into_iter()
        .take(selectable_len)
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

        let request = select_compaction_after_token_trigger(&turns, Some("summary".to_string()))
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

        assert!(select_compaction_after_token_trigger(&turns, None).is_none());
    }

    #[test]
    fn token_trigger_selects_at_least_one_old_turn() {
        let turns = vec![
            completed_turn(1, 10),
            completed_turn(2, 10),
            completed_turn(3, 10),
        ];

        let request =
            select_compaction_after_token_trigger(&turns, None).expect("compaction request");

        assert_eq!(request.compact_turn_ids, vec!["turn_1".to_string()]);
    }
}
