use super::*;
use crate::render::ToolCallDisplayMode;

#[test]
fn lifecycle_view_replaces_call_with_result() {
    let mut view = ToolView::running(
        "read_file".to_string(),
        r#"{"path":"README.md"}"#.to_string(),
    );
    view.set_progress("reading file".to_string());
    view.finish(true, "contents".to_string());

    let output = render(&view, ToolCallDisplayMode::Full);

    assert!(output.contains("README.md"));
    assert!(output.contains("reading file"));
    assert!(output.contains("contents"));
    assert!(output.contains("└─"));
}

#[test]
fn summary_view_keeps_failure_visible() {
    let output = render_result(
        "read_file",
        false,
        "permission denied",
        ToolCallDisplayMode::Summary,
    );

    assert!(!output.is_empty());
    assert!(output.contains("err"));
}

#[test]
fn todo_result_renders_items_instead_of_raw_json() {
    let output = render_result(
        "todo",
        true,
        r#"{"ok":true,"items":[{"id":"1","text":"检查测试","status":"completed"},{"id":"2","text":"构建项目","status":"in_progress"}]}"#,
        ToolCallDisplayMode::Full,
    );

    assert!(output.contains("[x] 检查测试"));
    assert!(output.contains("[>] 构建项目"));
    assert!(!output.contains("\"items\""));
}
