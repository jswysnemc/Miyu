use crate::render::edit_diff::render_edit_file_diff;
use crate::render::tool_event_line::{tool_event_label, tool_event_text};

/// edit_file 调用的原始 patch 参数。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DiffCell {
    pub(crate) arguments: String,
}

/// 依据原始 patch 参数重建 diff 视图。
///
/// 参数:
/// - `cell`: diff 源数据
///
/// 返回:
/// - ANSI 文本块
pub(crate) fn render(cell: &DiffCell) -> String {
    render_edit_file_diff(&cell.arguments)
        .unwrap_or_else(|| {
            tool_event_text(&tool_event_label("edit_file", Some(&cell.arguments)), "run")
        })
        .trim_end()
        .to_string()
}
