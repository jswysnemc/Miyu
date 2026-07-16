use super::*;

#[test]
fn session_memory_schema_is_created() {
    let temp = tempfile::tempdir().unwrap();
    let paths = test_paths(temp.path().to_path_buf());

    let store = StateStore::new(&paths).unwrap();

    let conn = rusqlite::Connection::open(store.state_dir.join("conversation.db")).unwrap();
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'session_memory'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(exists, 1);
}

#[test]
fn session_snapshot_includes_session_memory_summary() {
    let temp = tempfile::tempdir().unwrap();
    let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
    crate::state::session_memory::repository::upsert_memory(
        &store.conv_db,
        crate::state::session_memory::model::NewSessionMemory {
            session_id: store.session_id().to_string(),
            summary: "current task, constraints, and next step".to_string(),
            last_summarized_turn_id: Some("turn_3".to_string()),
            last_summarized_seq: 3,
            checkpoint_id: Some("checkpoint_1".to_string()),
            source_turn_count: 3,
            token_estimate: 128,
        },
    )
    .unwrap();

    let snapshot = store.session_snapshot(1_000).unwrap();

    let memory = snapshot.session_memory.expect("session memory summary");
    assert_eq!(memory.last_summarized_seq, 3);
    assert_eq!(memory.last_summarized_turn_id.as_deref(), Some("turn_3"));
    assert_eq!(memory.checkpoint_id.as_deref(), Some("checkpoint_1"));
    assert_eq!(memory.source_turn_count, 3);
    assert_eq!(memory.token_estimate, 128);
    assert!(!memory.is_disabled);
}

#[test]
fn complete_turn_updates_session_memory() {
    let temp = tempfile::tempdir().unwrap();
    let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
    store.start_turn("turn_1", "remember this task").unwrap();
    store
        .complete_turn("turn_1", "remember this answer", None)
        .unwrap();

    let snapshot = store.session_snapshot(1_000).unwrap();
    let memory = snapshot.session_memory.expect("session memory summary");

    assert_eq!(memory.last_summarized_turn_id.as_deref(), Some("turn_1"));
    assert_eq!(memory.last_summarized_seq, 1);
    assert_eq!(memory.source_turn_count, 1);
    assert!(memory.token_estimate > 0);
}

#[test]
fn auto_compaction_prefers_session_memory_input_when_boundary_valid() {
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
    crate::state::session_memory::repository::upsert_memory(
        &store.conv_db,
        crate::state::session_memory::model::NewSessionMemory {
            session_id: store.session_id().to_string(),
            summary: "memory first summary".to_string(),
            last_summarized_turn_id: Some("turn_2".to_string()),
            last_summarized_seq: 2,
            checkpoint_id: None,
            source_turn_count: 2,
            token_estimate: 64,
        },
    )
    .unwrap();
    let messages = vec![ChatMessage::plain("user", "x".repeat(8_000))];

    let request = store
        .select_compaction_for_messages(&messages, 2_000, 0.5, 0.5)
        .unwrap()
        .expect("memory compact request");

    assert_eq!(
        request.compact_turn_ids,
        vec!["turn_3".to_string(), "turn_4".to_string()]
    );
    assert_eq!(
        request.previous_summary.as_deref(),
        Some("memory first summary")
    );

    store
        .apply_compaction(&request, "checkpoint from memory")
        .unwrap();
    let turns = store.load_turns().unwrap();
    let projected = store.project_history(None).unwrap();

    assert!(turns.is_empty());
    assert_eq!(projected.stats.covered_turns, 4);
    assert!(projected
        .checkpoint_context
        .as_deref()
        .unwrap()
        .contains("covered_from_seq=\"1\" covered_to_seq=\"4\""));
}

#[test]
fn invalid_memory_boundary_records_recovery_and_falls_back_to_checkpoint_compact() {
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
    crate::state::session_memory::repository::upsert_memory(
        &store.conv_db,
        crate::state::session_memory::model::NewSessionMemory {
            session_id: store.session_id().to_string(),
            summary: "stale invalid summary".to_string(),
            last_summarized_turn_id: Some("turn_99".to_string()),
            last_summarized_seq: 99,
            checkpoint_id: None,
            source_turn_count: 99,
            token_estimate: 64,
        },
    )
    .unwrap();
    let messages = vec![ChatMessage::plain("user", "x".repeat(8_000))];

    let request = store
        .select_compaction_for_messages(&messages, 2_000, 0.5, 0.5)
        .unwrap()
        .expect("fallback compaction request");
    let recovery = store.recovery_snapshot().unwrap();

    assert_eq!(
        request.compact_turn_ids,
        vec!["turn_1".to_string(), "turn_2".to_string()]
    );
    assert_eq!(
        recovery.latest.as_ref().unwrap().kind,
        FailureKind::SessionMemoryBoundaryInvalid
    );
    assert!(recovery
        .latest
        .as_ref()
        .unwrap()
        .reason
        .contains("boundary_after_latest_turn"));
}

#[test]
fn memory_compact_over_budget_records_recovery_without_checkpoint() {
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
    crate::state::session_memory::repository::upsert_memory(
        &store.conv_db,
        crate::state::session_memory::model::NewSessionMemory {
            session_id: store.session_id().to_string(),
            summary: "memory first summary".to_string(),
            last_summarized_turn_id: Some("turn_2".to_string()),
            last_summarized_seq: 2,
            checkpoint_id: None,
            source_turn_count: 2,
            token_estimate: 64,
        },
    )
    .unwrap();
    let messages = vec![ChatMessage::plain("user", "x".repeat(8_000))];
    let request = store
        .select_compaction_for_messages(&messages, 2_000, 0.5, 0.5)
        .unwrap()
        .expect("memory compact request");
    let projection =
        crate::state::request_projection::project_provider_turn_from_messages(&messages, 0, 500);

    let outcome = store
        .apply_compaction_with_budget_guard(&request, &"s".repeat(20_000), &projection, None)
        .unwrap();
    let recovery = store.recovery_snapshot().unwrap();
    let projected = store.project_history(None).unwrap();

    assert_eq!(outcome, CompactionApplyOutcome::RejectedOverBudget);
    assert_eq!(projected.stats.checkpoint_count, 0);
    assert_eq!(
        recovery.latest.as_ref().unwrap().kind,
        FailureKind::CompactionOverBudget
    );
    assert!(recovery
        .latest
        .as_ref()
        .unwrap()
        .reason
        .contains("compaction reprojected provider request over budget"));
}

#[test]
fn legacy_selector_bypasses_memory_after_memory_compact_over_budget() {
    let temp = tempfile::tempdir().unwrap();
    let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
    for index in 1..=4 {
        let turn_id = format!("turn_{index}");
        store.start_turn(&turn_id, &"u".repeat(200)).unwrap();
        store
            .complete_turn(&turn_id, &"a".repeat(200), None)
            .unwrap();
    }
    crate::state::session_memory::repository::upsert_memory(
        &store.conv_db,
        crate::state::session_memory::model::NewSessionMemory {
            session_id: store.session_id().to_string(),
            summary: "memory first summary".to_string(),
            last_summarized_turn_id: Some("turn_2".to_string()),
            last_summarized_seq: 2,
            checkpoint_id: None,
            source_turn_count: 2,
            token_estimate: 64,
        },
    )
    .unwrap();
    let messages = vec![ChatMessage::plain("user", "x".repeat(8_000))];
    let memory_request = store
        .select_compaction_for_messages(&messages, 2_000, 0.5, 0.5)
        .unwrap()
        .expect("memory compact request");
    assert_eq!(
        memory_request.compact_turn_ids,
        vec!["turn_3".to_string(), "turn_4".to_string()]
    );

    let legacy_request = store
        .select_legacy_compaction_for_messages(&messages, 2_000, 0.5, 0.5)
        .unwrap()
        .expect("legacy compact request");

    assert_eq!(
        legacy_request.compact_turn_ids,
        vec!["turn_1".to_string(), "turn_2".to_string()]
    );
    assert_ne!(
        legacy_request.previous_summary.as_deref(),
        Some("memory first summary")
    );
}

#[test]
fn reset_conversation_clears_session_memory() {
    let temp = tempfile::tempdir().unwrap();
    let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
    store.start_turn("turn_1", "remember this task").unwrap();
    store
        .complete_turn("turn_1", "remember this answer", None)
        .unwrap();
    assert!(store
        .session_snapshot(1_000)
        .unwrap()
        .session_memory
        .is_some());

    store.reset_conversation().unwrap();

    assert!(store
        .session_snapshot(1_000)
        .unwrap()
        .session_memory
        .is_none());
}
