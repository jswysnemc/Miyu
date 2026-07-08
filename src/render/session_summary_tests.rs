use super::session_summary::render_session_summary;
use crate::runtime_recovery::{RuntimeRecoveryKind, RuntimeRecoveryStatus, RuntimeRecoverySummary};
use crate::state::SessionSnapshot;
use crate::state::{ContextEpochSummary, SessionMemorySummary};
use crate::state::{ToolCallStatus, ToolHistorySummary, UsageSnapshot};

#[test]
fn renders_session_summary_card_with_usage_and_compaction() {
    let snapshot = SessionSnapshot {
        session_id: "default".to_string(),
        turn_count: 2,
        context_chars: 100,
        context_limit_chars: 1_000,
        context_ratio: 0.1,
        checkpoint_count: 0,
        checkpoint_covered_turns: 0,
        tail_turns: 2,
        latest_checkpoint_at: None,
        usage: UsageSnapshot {
            requests: 1,
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            last_usage: Some(crate::llm::Usage {
                prompt_tokens: 7,
                completion_tokens: 3,
                total_tokens: 10,
            }),
        },
        compaction: Some(crate::state::CompactionSummary {
            summary: "summary".to_string(),
            compacted_turns: 4,
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }),
        recovery: crate::state::RecoverySnapshot::default(),
        context_epoch: None,
        session_memory: None,
        tool_history: ToolHistorySummary::default(),
        runtime_recovery: crate::runtime_recovery::RuntimeRecoverySummary::default(),
        dynamic_sources: Vec::new(),
        projection_warnings: Vec::new(),
        active_run: None,
    };

    let output = render_session_summary(&snapshot);

    assert!(output.contains("Miyu"));
    assert!(output.contains("default"));
    assert!(output.contains("10.0%"));
    assert!(output.contains("1"));
    assert!(output.contains("15"));
    assert!(output.contains("4"));
    assert!(output.contains("2026-01-01T00:00:00Z"));
}

#[test]
fn renders_checkpoint_coverage() {
    let snapshot = SessionSnapshot {
        session_id: "default".to_string(),
        turn_count: 4,
        context_chars: 100,
        context_limit_chars: 1_000,
        context_ratio: 0.1,
        checkpoint_count: 1,
        checkpoint_covered_turns: 3,
        tail_turns: 1,
        latest_checkpoint_at: Some("2026-01-01T00:00:00Z".to_string()),
        usage: UsageSnapshot::default(),
        compaction: None,
        recovery: crate::state::RecoverySnapshot::default(),
        context_epoch: None,
        session_memory: None,
        tool_history: ToolHistorySummary::default(),
        runtime_recovery: crate::runtime_recovery::RuntimeRecoverySummary::default(),
        dynamic_sources: Vec::new(),
        projection_warnings: Vec::new(),
        active_run: None,
    };

    let output = render_session_summary(&snapshot);

    assert!(output.contains("Checkpoint") || output.contains("检查点"));
    assert!(output.contains("3"));
    assert!(output.contains("2026-01-01T00:00:00Z"));
}

#[test]
fn renders_recovery_warning_when_present() {
    let snapshot = SessionSnapshot {
        session_id: "default".to_string(),
        turn_count: 4,
        context_chars: 900,
        context_limit_chars: 1_000,
        context_ratio: 0.9,
        checkpoint_count: 1,
        checkpoint_covered_turns: 3,
        tail_turns: 1,
        latest_checkpoint_at: Some("2026-01-01T00:00:00Z".to_string()),
        usage: UsageSnapshot::default(),
        compaction: None,
        recovery: crate::state::RecoverySnapshot {
            latest: None,
            auto_compaction_failures: 3,
            auto_compaction_blocked: true,
            stale_turns_recovered: 0,
        },
        context_epoch: None,
        session_memory: None,
        tool_history: ToolHistorySummary::default(),
        runtime_recovery: crate::runtime_recovery::RuntimeRecoverySummary::default(),
        dynamic_sources: Vec::new(),
        projection_warnings: Vec::new(),
        active_run: None,
    };

    let output = render_session_summary(&snapshot);

    assert!(output.contains("Recovery") || output.contains("恢复"));
    assert!(output.contains("自动压缩已熔断"));
}

#[test]
fn renders_context_epoch_summary_when_present() {
    let snapshot = SessionSnapshot {
        session_id: "default".to_string(),
        turn_count: 1,
        context_chars: 100,
        context_limit_chars: 1_000,
        context_ratio: 0.1,
        checkpoint_count: 0,
        checkpoint_covered_turns: 0,
        tail_turns: 1,
        latest_checkpoint_at: None,
        usage: UsageSnapshot::default(),
        compaction: None,
        recovery: crate::state::RecoverySnapshot::default(),
        context_epoch: Some(ContextEpochSummary {
            baseline_hash: "abcdef1234567890".to_string(),
            source_count: 1,
            last_change_reason: "initialized".to_string(),
            blocked_source: None,
        }),
        session_memory: None,
        tool_history: ToolHistorySummary::default(),
        runtime_recovery: crate::runtime_recovery::RuntimeRecoverySummary::default(),
        dynamic_sources: Vec::new(),
        projection_warnings: Vec::new(),
        active_run: None,
    };

    let output = render_session_summary(&snapshot);

    assert!(output.contains("Context Epoch") || output.contains("上下文纪元"));
    assert!(output.contains("abcdef12"));
    assert!(output.contains("initialized"));
}

#[test]
fn renders_dynamic_context_sources_when_present() {
    let snapshot = SessionSnapshot {
        session_id: "default".to_string(),
        turn_count: 1,
        context_chars: 64,
        context_limit_chars: 1_000,
        context_ratio: 0.064,
        checkpoint_count: 0,
        checkpoint_covered_turns: 0,
        tail_turns: 1,
        latest_checkpoint_at: None,
        usage: UsageSnapshot::default(),
        compaction: None,
        recovery: crate::state::RecoverySnapshot::default(),
        context_epoch: None,
        session_memory: None,
        tool_history: ToolHistorySummary::default(),
        runtime_recovery: crate::runtime_recovery::RuntimeRecoverySummary::default(),
        dynamic_sources: vec![crate::state::request_projection::DynamicContextSource {
            key: "runtime_context".to_string(),
            chars: 42,
        }],
        projection_warnings: Vec::new(),
        active_run: None,
    };

    let output = render_session_summary(&snapshot);

    assert!(output.contains("Dynamic context") || output.contains("动态上下文"));
    assert!(
        output.contains("runtime_context 42 chars") || output.contains("runtime_context 42 字符")
    );
}

#[test]
fn renders_projection_warnings_when_present() {
    let snapshot = SessionSnapshot {
        session_id: "default".to_string(),
        turn_count: 1,
        context_chars: 0,
        context_limit_chars: 0,
        context_ratio: 0.0,
        checkpoint_count: 0,
        checkpoint_covered_turns: 0,
        tail_turns: 1,
        latest_checkpoint_at: None,
        usage: UsageSnapshot::default(),
        compaction: None,
        recovery: crate::state::RecoverySnapshot::default(),
        context_epoch: None,
        session_memory: None,
        tool_history: ToolHistorySummary::default(),
        runtime_recovery: crate::runtime_recovery::RuntimeRecoverySummary::default(),
        dynamic_sources: Vec::new(),
        projection_warnings: vec![
            "session summary projection has invalid context limit".to_string()
        ],
        active_run: None,
    };

    let output = render_session_summary(&snapshot);

    assert!(output.contains("Projection") || output.contains("投影"));
    assert!(output.contains("invalid context limit"));
}

#[test]
fn renders_session_memory_summary_when_present() {
    let snapshot = SessionSnapshot {
        session_id: "default".to_string(),
        turn_count: 3,
        context_chars: 128,
        context_limit_chars: 1_000,
        context_ratio: 0.128,
        checkpoint_count: 1,
        checkpoint_covered_turns: 2,
        tail_turns: 1,
        latest_checkpoint_at: Some("2026-01-01T00:00:00Z".to_string()),
        usage: UsageSnapshot::default(),
        compaction: None,
        recovery: crate::state::RecoverySnapshot::default(),
        context_epoch: None,
        session_memory: Some(SessionMemorySummary {
            last_summarized_turn_id: Some("turn_3".to_string()),
            last_summarized_seq: 3,
            checkpoint_id: Some("checkpoint_1".to_string()),
            source_turn_count: 3,
            token_estimate: 128,
            consecutive_failures: 0,
            is_disabled: false,
            last_error: None,
            updated_at: "2026-01-01T00:00:01Z".to_string(),
        }),
        tool_history: ToolHistorySummary::default(),
        runtime_recovery: crate::runtime_recovery::RuntimeRecoverySummary::default(),
        dynamic_sources: Vec::new(),
        projection_warnings: Vec::new(),
        active_run: None,
    };

    let output = render_session_summary(&snapshot);

    assert!(output.contains("Session Memory") || output.contains("会话记忆"));
    assert!(output.contains("seq 3") || output.contains("序号 3"));
    assert!(output.contains("turn_3"));
    assert!(output.contains("checkpoint_1"));
    assert!(output.contains("128"));
}

#[test]
fn renders_tool_history_summary_when_present() {
    let snapshot = SessionSnapshot {
        session_id: "default".to_string(),
        turn_count: 3,
        context_chars: 128,
        context_limit_chars: 1_000,
        context_ratio: 0.128,
        checkpoint_count: 0,
        checkpoint_covered_turns: 0,
        tail_turns: 3,
        latest_checkpoint_at: None,
        usage: UsageSnapshot::default(),
        compaction: None,
        recovery: crate::state::RecoverySnapshot::default(),
        context_epoch: None,
        session_memory: None,
        tool_history: ToolHistorySummary {
            call_count: 2,
            result_count: 2,
            pending_count: 0,
            error_count: 1,
            replacement_count: 1,
            latest_tool_name: Some("read_file".to_string()),
            latest_status: Some(ToolCallStatus::Completed),
        },
        runtime_recovery: crate::runtime_recovery::RuntimeRecoverySummary::default(),
        dynamic_sources: Vec::new(),
        projection_warnings: Vec::new(),
        active_run: None,
    };

    let output = render_session_summary(&snapshot);

    assert!(output.contains("Tool History") || output.contains("工具历史"));
    assert!(output.contains("read_file"));
    assert!(output.contains("2"));
    assert!(output.contains("1"));
}

#[test]
fn renders_active_run_summary_when_present() {
    let snapshot = SessionSnapshot {
        session_id: "default".to_string(),
        turn_count: 1,
        context_chars: 128,
        context_limit_chars: 1_000,
        context_ratio: 0.128,
        checkpoint_count: 0,
        checkpoint_covered_turns: 0,
        tail_turns: 1,
        latest_checkpoint_at: None,
        usage: UsageSnapshot::default(),
        compaction: None,
        recovery: crate::state::RecoverySnapshot::default(),
        context_epoch: None,
        session_memory: None,
        tool_history: ToolHistorySummary::default(),
        runtime_recovery: crate::runtime_recovery::RuntimeRecoverySummary::default(),
        dynamic_sources: Vec::new(),
        projection_warnings: Vec::new(),
        active_run: Some(crate::state::ActiveRunSummary {
            owner: "command".to_string(),
            pid: 12345,
            started_at: "2026-01-01T00:00:00Z".to_string(),
            lock_path: "/tmp/miyu/active-run.json".to_string(),
        }),
    };

    let output = render_session_summary(&snapshot);

    assert!(output.contains("Active run") || output.contains("运行状态"));
    assert!(output.contains("command"));
    assert!(output.contains("12345"));
    assert!(output.contains("active-run.json"));
}

#[test]
fn renders_runtime_recovery_summary_when_present() {
    let snapshot = SessionSnapshot {
        session_id: "default".to_string(),
        turn_count: 1,
        context_chars: 128,
        context_limit_chars: 1_000,
        context_ratio: 0.128,
        checkpoint_count: 0,
        checkpoint_covered_turns: 0,
        tail_turns: 1,
        latest_checkpoint_at: None,
        usage: UsageSnapshot::default(),
        compaction: None,
        recovery: crate::state::RecoverySnapshot::default(),
        context_epoch: None,
        session_memory: None,
        tool_history: ToolHistorySummary::default(),
        runtime_recovery: RuntimeRecoverySummary {
            active_process_count: 1,
            stale_process_count: 1,
            latest_failure: Some(crate::runtime_recovery::RuntimeRecoveryFailureSummary {
                process_id: Some("proc_1".to_string()),
                kind: RuntimeRecoveryKind::SequenceGap,
                status: RuntimeRecoveryStatus::Terminal,
                reason: "missing seq 8".to_string(),
                last_safe_seq: Some(7),
                created_at: "2026-01-01T00:00:00Z".to_string(),
            }),
        },
        dynamic_sources: Vec::new(),
        projection_warnings: Vec::new(),
        active_run: None,
    };

    let output = render_session_summary(&snapshot);

    assert!(output.contains("Runtime Recovery") || output.contains("运行时恢复"));
    assert!(output.contains("proc_1"));
    assert!(output.contains("missing seq 8"));
    assert!(output.contains("7"));
}
