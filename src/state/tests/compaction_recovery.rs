use super::*;
use crate::llm::ChatMessage;

#[test]
fn compaction_failure_records_recovery_without_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
    store.start_turn("turn_1", "hello").unwrap();
    store.complete_turn("turn_1", "hi", None).unwrap();

    store
        .record_auto_compaction_failure(
            Some("turn_1"),
            FailureKind::CompactionLlmFailed,
            "provider error",
            900,
            1_000,
        )
        .unwrap();

    let turns = store.load_turns().unwrap();
    let projected = store.project_history(None).unwrap();
    let recovery = store.recovery_snapshot().unwrap();

    assert_eq!(turns.len(), 1);
    assert_eq!(projected.stats.checkpoint_count, 0);
    assert_eq!(recovery.auto_compaction_failures, 1);
    assert_eq!(
        recovery.latest.as_ref().unwrap().kind,
        FailureKind::CompactionLlmFailed
    );
}

#[test]
fn empty_summary_does_not_replace_previous_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
    for index in 1..=4 {
        let turn_id = format!("turn_{index}");
        store
            .start_turn(&turn_id, &format!("user {index}"))
            .unwrap();
        store
            .complete_turn(&turn_id, &format!("assistant {index}"), None)
            .unwrap();
    }
    let request = store.select_manual_compaction(1).unwrap().unwrap();
    store
        .apply_compaction(&request, "stable checkpoint")
        .unwrap();
    let before = store.project_history(None).unwrap();

    store
        .record_auto_compaction_failure(
            None,
            FailureKind::EmptySummary,
            "compaction summary is empty",
            950,
            1_000,
        )
        .unwrap();
    let after = store.project_history(None).unwrap();
    let recovery = store.recovery_snapshot().unwrap();

    assert_eq!(after.stats.checkpoint_count, before.stats.checkpoint_count);
    assert_eq!(after.stats.covered_turns, before.stats.covered_turns);
    assert!(after
        .checkpoint_context
        .as_ref()
        .unwrap()
        .contains("stable checkpoint"));
    assert_eq!(
        recovery.latest.as_ref().unwrap().kind,
        FailureKind::EmptySummary
    );
}

#[test]
fn compaction_budget_check_does_not_write_checkpoint_or_recovery() {
    let temp = tempfile::tempdir().unwrap();
    let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
    for index in 1..=4 {
        let turn_id = format!("turn_{index}");
        store.start_turn(&turn_id, &"u".repeat(200)).unwrap();
        store
            .complete_turn(&turn_id, &"a".repeat(200), None)
            .unwrap();
    }
    let messages = vec![ChatMessage::plain("user", "x".repeat(1_800))];
    let request = store
        .select_compaction_for_messages(&messages, 2_000, 0.5, 0.5)
        .unwrap()
        .expect("compaction request");
    let projection =
        crate::state::request_projection::project_provider_turn_from_messages(&messages, 0, 2_000);

    let budget = store
        .compaction_budget_check(&request, &"s".repeat(3_000), &projection, None)
        .unwrap();
    let projected = store.project_history(None).unwrap();
    let recovery = store.recovery_snapshot().unwrap();

    assert!(budget.is_over_budget());
    assert_eq!(projected.stats.checkpoint_count, 0);
    assert_eq!(recovery.auto_compaction_failures, 0);
    assert!(recovery.latest.is_none());
}

#[test]
fn manual_compaction_over_budget_records_failure_without_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
    for index in 1..=4 {
        let turn_id = format!("turn_{index}");
        store.start_turn(&turn_id, &"u".repeat(200)).unwrap();
        store
            .complete_turn(&turn_id, &"a".repeat(200), None)
            .unwrap();
    }
    let request = store.select_manual_compaction(1).unwrap().unwrap();

    let outcome = store
        .apply_manual_compaction_with_budget_guard(&request, &"s".repeat(3_000), 2_000)
        .unwrap();
    let projected = store.project_history(None).unwrap();
    let recovery = store.recovery_snapshot().unwrap();

    assert_eq!(outcome, CompactionApplyOutcome::RejectedOverBudget);
    assert_eq!(projected.stats.checkpoint_count, 0);
    assert_eq!(recovery.auto_compaction_failures, 0);
    assert_eq!(
        recovery.latest.as_ref().unwrap().kind,
        FailureKind::CompactionOverBudget
    );
    assert!(recovery
        .latest
        .as_ref()
        .unwrap()
        .reason
        .contains("manual compaction reprojected history over budget"));
}

#[test]
fn auto_compaction_circuit_breaker_skips_after_three_failures() {
    let temp = tempfile::tempdir().unwrap();
    let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
    for index in 1..=3 {
        store
            .record_auto_compaction_failure(
                None,
                FailureKind::CompactionLlmFailed,
                &format!("provider error {index}"),
                900,
                1_000,
            )
            .unwrap();
    }

    let recovery = store.recovery_snapshot().unwrap();

    assert_eq!(recovery.auto_compaction_failures, 3);
    assert!(recovery.auto_compaction_blocked);
    assert!(!store.should_attempt_auto_compaction().unwrap());
}

#[test]
fn manual_compaction_failure_does_not_trip_auto_breaker() {
    let temp = tempfile::tempdir().unwrap();
    let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
    for index in 1..=3 {
        store
            .record_manual_compaction_failure(
                FailureKind::CompactionLlmFailed,
                &format!("manual error {index}"),
                900,
                1_000,
            )
            .unwrap();
    }

    let recovery = store.recovery_snapshot().unwrap();

    assert_eq!(recovery.auto_compaction_failures, 0);
    assert!(!recovery.auto_compaction_blocked);
    assert!(store.should_attempt_auto_compaction().unwrap());
}
