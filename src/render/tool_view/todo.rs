use super::model::ToolView;
use crate::render::tool_event_line::{tool_event_label, tool_event_text};
use crate::render::ToolCallDisplayMode;
use serde::Deserialize;

/// TODO 工具返回的待办条目。
#[derive(Deserialize)]
struct TodoItemView {
    text: String,
    status: String,
}

/// TODO 工具的结构化返回。
#[derive(Deserialize)]
struct TodoResultView {
    items: Vec<TodoItemView>,
}

/// 渲染 TODO 工具的结构化清单。
///
/// 参数:
/// - `view`: TODO 工具生命周期
/// - `mode`: 工具展示模式
///
/// 返回:
/// - 可识别时返回待办清单文本
pub(super) fn render(view: &ToolView, mode: ToolCallDisplayMode) -> Option<String> {
    if mode == ToolCallDisplayMode::Hidden {
        return Some(String::new());
    }
    let outcome = view.outcome.as_ref()?;
    let result = serde_json::from_str::<TodoResultView>(&outcome.output).ok()?;
    let label = tool_event_label("todo", Some(&view.arguments));
    let mut output = tool_event_text(&label, if outcome.ok { "ok" } else { "err" });
    for item in result.items {
        output.push_str("\n  ");
        output.push_str(status_marker(&item.status));
        output.push(' ');
        output.push_str(&item.text);
    }
    Some(output)
}

/// 返回待办状态的纯文本标记。
///
/// 参数:
/// - `status`: 待办状态
///
/// 返回:
/// - 固定宽度状态标记
fn status_marker(status: &str) -> &'static str {
    match status {
        "completed" => "[x]",
        "in_progress" => "[>]",
        "cancelled" => "[-]",
        _ => "[ ]",
    }
}
