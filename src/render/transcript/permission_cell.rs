use crate::permission::{PermissionDecision, PermissionRequest};

/// TUI transcript 中已经处理的权限事件。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PermissionCell {
    pub(crate) request: PermissionRequest,
    pub(crate) decision: Option<PermissionDecision>,
    pub(crate) reply_draft: Option<String>,
}

impl PermissionCell {
    /// 创建等待用户选择的权限事件。
    ///
    /// 参数:
    /// - `request`: 权限请求
    ///
    /// 返回:
    /// - 尚未包含决定的权限 cell
    pub(crate) fn pending(request: PermissionRequest) -> Self {
        Self {
            request,
            decision: None,
            reply_draft: None,
        }
    }

    /// 写入用户对当前权限请求的最终决定。
    ///
    /// 参数:
    /// - `decision`: 用户决定
    ///
    /// 返回:
    /// - 无
    pub(crate) fn resolve(&mut self, decision: PermissionDecision) {
        self.decision = Some(decision);
        self.reply_draft = None;
    }

    /// 更新拒绝回复的内联编辑草稿。
    ///
    /// 参数:
    /// - `draft`: 回复草稿；空值表示返回权限选择
    ///
    /// 返回:
    /// - 无
    pub(crate) fn set_reply_draft(&mut self, draft: Option<String>) {
        self.reply_draft = draft;
    }
}

/// 渲染权限事件。
///
/// 参数:
/// - `cell`: 权限请求与用户决定
///
/// 返回:
/// - ANSI 权限事件文本
pub(crate) fn render(cell: &PermissionCell) -> String {
    match &cell.decision {
        Some(decision) => crate::render::render_permission_event(&cell.request, decision),
        None => {
            let mut output = crate::render::render_permission_prompt(&cell.request);
            if let Some(draft) = &cell.reply_draft {
                output.push_str("\n  \x1b[2m回复 Miyu\x1b[0m\n    ");
                output.push_str(draft);
                output.push_str("\x1b[36m▌\x1b[0m\n  \x1b[2mEnter 提交 · Esc 返回\x1b[0m");
            }
            output
        }
    }
}
