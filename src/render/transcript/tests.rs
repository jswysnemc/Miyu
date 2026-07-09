use super::line::AnsiLine;
use super::{TranscriptMode, TranscriptRenderOptions, TranscriptStore};
use crate::llm::{ChatStreamChunk, ChatStreamKind, ToolCallStreamProgress};
use crate::render::{ReasoningDisplayMode, ToolCallDisplayMode};

fn options() -> TranscriptRenderOptions {
    TranscriptRenderOptions {
        reasoning_mode: ReasoningDisplayMode::Full,
        tool_call_mode: ToolCallDisplayMode::Summary,
    }
}

fn chunk(kind: ChatStreamKind, text: &str) -> ChatStreamChunk {
    ChatStreamChunk {
        kind,
        text: text.to_string(),
    }
}

#[test]
fn ansi_lines_are_prewrapped_at_requested_width() {
    let lines = AnsiLine::wrap_block("\x1b[31mabcdef\x1b[0m", 3);

    assert_eq!(lines.len(), 2);
    assert!(lines[0].as_str().contains("abc"));
    assert!(lines[1].as_str().contains("def"));
    assert!(lines.iter().all(|line| line.as_str().ends_with("\x1b[0m")));
}

#[test]
fn live_tail_is_visible_before_consolidation_and_retained_afterward() {
    let mut store = TranscriptStore::new(100);
    store.push_user_echo(TranscriptMode::Yolo, "inspect resize".to_string());
    store.push_chunk(&chunk(ChatStreamKind::Content, "streamed answer\n"));

    assert_eq!(store.display_live_tail(80, &options()).len(), 1);
    assert!(store
        .display_tail(80, &options())
        .iter()
        .any(|line| line.as_str().contains("streamed answer")));

    assert!(store.finalize_live_tail());
    assert!(store.display_live_tail(80, &options()).is_empty());
    assert!(store
        .display_tail(80, &options())
        .iter()
        .any(|line| line.as_str().contains("streamed answer")));
}

#[test]
fn live_table_is_emitted_once_without_cursor_replacement_sequences() {
    let mut store = TranscriptStore::new(100);
    store.push_chunk(&chunk(
        ChatStreamKind::Content,
        "| Tool | Purpose |\n| --- | --- |\n| read_file | Read files |\n",
    ));

    assert!(store.display_live_tail(80, &options()).is_empty());

    store.push_chunk(&chunk(ChatStreamKind::Content, "complete\n"));
    let lines = store.display_live_tail(80, &options());
    let rendered = lines.iter().map(|line| line.as_str()).collect::<String>();

    assert!(rendered.contains('┌'));
    assert!(rendered.contains("read_file"));
    assert!(!rendered.contains("\x1b[1A"));
}

#[test]
fn live_reasoning_summary_animates_without_waiting_for_consolidation() {
    let mut store = TranscriptStore::new(100);
    store.push_chunk(&chunk(ChatStreamKind::Reasoning, "inspect resize"));

    let first = store.display_live_tail(80, &options());
    assert!(store.advance_live_animation());
    let second = store.display_live_tail(80, &options());

    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_ne!(first, second);
    assert!(second[0].as_str().contains("14 chars"));
}

#[test]
fn live_tool_argument_preview_is_visible_until_the_call_is_finalized() {
    let mut store = TranscriptStore::new(100);
    store.push_tool_call_progress(&ToolCallStreamProgress {
        index: 0,
        name: Some("read_file".to_string()),
        arguments_chars: 12,
        arguments_bytes: 12,
        arguments_preview: r#"{"path":"REA"#.to_string(),
    });

    assert!(store
        .display_live_tail(80, &options())
        .iter()
        .any(|line| line.as_str().contains("Read")));

    store.push_tool_call(
        "read_file".to_string(),
        r#"{"path":"README.md"}"#.to_string(),
    );
    assert!(store.display_live_tail(80, &options()).is_empty());
    assert!(store
        .display_tail(80, &options())
        .iter()
        .any(|line| line.as_str().contains("README.md")));
}

#[test]
fn user_echo_uses_a_prominent_bullet() {
    let mut store = TranscriptStore::new(100);
    store.push_user_echo(TranscriptMode::Yolo, "inspect resize".to_string());

    assert!(store
        .display_tail(80, &options())
        .iter()
        .any(|line| line.as_str().contains("●")));
}

#[test]
fn summary_mode_keeps_compact_tool_call_block_visible() {
    let mut store = TranscriptStore::new(100);
    store.push_tool_call(
        "read_file".to_string(),
        r#"{"path":"README.md"}"#.to_string(),
    );

    let lines = store.display_tail(80, &options());

    assert!(!lines.is_empty());
    assert!(lines.iter().any(|line| line.as_str().contains("Read")));
}

#[test]
fn summary_mode_keeps_tool_progress_message_visible() {
    let mut store = TranscriptStore::new(100);
    store.push_tool_progress(
        "task".to_string(),
        "subagent is checking the implementation".to_string(),
    );

    let lines = store.display_tail(80, &options());

    assert!(lines
        .iter()
        .any(|line| line.as_str().contains("subagent is checking")));
}

#[test]
fn row_cap_trims_prewrapped_rows_not_source_cells() {
    let mut store = TranscriptStore::new(2);
    store.push_meta("first".to_string());
    store.push_meta("second".to_string());
    store.push_meta("third".to_string());

    let lines = store.display_tail(80, &options());

    assert_eq!(lines.len(), 2);
    assert!(lines[0].as_str().contains("second"));
    assert!(lines[1].as_str().contains("third"));
}

#[test]
fn diff_fill_is_reapplied_to_each_prewrapped_row() {
    let lines = AnsiLine::wrap_block("\x1b[48;5;22mabcdef\x1b[K\x1b[0m", 3);

    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|line| line.as_str().contains("\x1b[K")));
}
