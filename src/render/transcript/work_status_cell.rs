use crate::render::work_status::WorkStatus;

/// 渲染 REPL 当前工作状态。
///
/// 参数:
/// - `status`: 当前单轮工作状态
///
/// 返回:
/// - 带 ANSI 样式的状态行
pub(super) fn render(status: WorkStatus) -> String {
    status.render_line()
}
