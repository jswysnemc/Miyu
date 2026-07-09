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
