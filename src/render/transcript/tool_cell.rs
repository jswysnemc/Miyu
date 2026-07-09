use crate::render::tool_event_line::tool_event_text;
use crate::render::tool_view::{self, ToolView};
use crate::render::ToolCallDisplayMode;

/// REPL 工具历史单元。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ToolCell {
    Invocation(ToolView),
    CompactionStarted { turn_count: usize },
    CompactionFinished { applied: bool },
}

/// 渲染工具历史单元。
///
/// 参数:
/// - `cell`: 工具历史数据
/// - `mode`: 工具展示模式
///
/// 返回:
/// - ANSI 工具视图文本
pub(crate) fn render(cell: &ToolCell, mode: ToolCallDisplayMode) -> String {
    match cell {
        ToolCell::Invocation(view) => tool_view::render(view, mode),
        ToolCell::CompactionStarted { turn_count } => {
            tool_event_text(&format!("compact context×{turn_count}"), "run")
        }
        ToolCell::CompactionFinished { applied } => {
            tool_event_text("compact context", if *applied { "ok" } else { "skip" })
        }
    }
}

/// 渲染尚未接收完整参数的工具调用。
///
/// 参数:
/// - `name`: 工具名称
/// - `arguments_preview`: 当前参数预览
/// - `mode`: 工具展示模式
///
/// 返回:
/// - 可重绘的临时工具视图
pub(crate) fn render_live_call(
    name: &str,
    arguments_preview: &str,
    mode: ToolCallDisplayMode,
) -> String {
    tool_view::render(
        &ToolView::preparing(name.to_string(), arguments_preview.to_string()),
        mode,
    )
}
