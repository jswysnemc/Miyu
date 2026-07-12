use crate::render::edit_diff::render_edit_file_diff;
use crate::render::tool_event_line::{tool_event_label, tool_event_text};

/// edit_file 调用时生成的 diff 快照。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DiffCell {
    rendered: String,
}

impl DiffCell {
    /// 在工具执行前构造 diff 快照。
    ///
    /// 参数:
    /// - `arguments`: edit_file 原始参数
    ///
    /// 返回:
    /// - 不依赖后续文件状态的 diff cell
    pub(crate) fn from_arguments(arguments: String) -> Self {
        let rendered = render_edit_file_diff(&arguments).unwrap_or_else(|| {
            tool_event_text(&tool_event_label("edit_file", Some(&arguments)), "run")
        });
        Self {
            rendered: rendered.trim_end().to_string(),
        }
    }
}

/// 渲染已固化的 diff 快照。
///
/// 参数:
/// - `cell`: diff 源数据
///
/// 返回:
/// - ANSI 文本块
pub(crate) fn render(cell: &DiffCell) -> String {
    cell.rendered.clone()
}
