use crate::permission::{PermissionDecision, PermissionPresentation, PermissionRequest};

/// 权限选择项索引。
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum PermissionChoice {
    Allow = 0,
    Deny = 1,
    DenyWithReply = 2,
}

impl PermissionChoice {
    /// 返回所有可选操作。
    pub(crate) fn all() -> [Self; 3] {
        [Self::Allow, Self::Deny, Self::DenyWithReply]
    }

    /// 从索引解析选择项，越界时回退到 Allow。
    pub(crate) fn from_index(index: usize) -> Self {
        match index {
            1 => Self::Deny,
            2 => Self::DenyWithReply,
            _ => Self::Allow,
        }
    }

    /// 返回 0 起始索引。
    pub(crate) fn index(self) -> usize {
        self as usize
    }

    /// 向上移动选择。
    pub(crate) fn prev(self) -> Self {
        Self::from_index(self.index().saturating_sub(1))
    }

    /// 向下移动选择。
    pub(crate) fn next(self) -> Self {
        Self::from_index((self.index() + 1).min(2))
    }

    /// 返回选项标签。
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Allow => "允许一次",
            Self::Deny => "拒绝",
            Self::DenyWithReply => "拒绝并告诉 Miyu 如何调整",
        }
    }
}

/// 渲染终端中的权限请求详情和选择项。
///
/// 参数:
/// - `request`: 待处理权限请求
/// - `selected`: 当前高亮选项
///
/// 返回:
/// - Codex 风格的 ANSI 文本块
pub(crate) fn render_permission_prompt(request: &PermissionRequest, selected: PermissionChoice) -> String {
    let mut output = render_request_body(request);
    output.push('\n');
    for choice in PermissionChoice::all() {
        let active = choice == selected;
        if active {
            output.push_str(&format!(
                "\n  \x1b[1;36m❯ {}\x1b[0m",
                choice.label()
            ));
        } else {
            output.push_str(&format!("\n    {}", choice.label()));
        }
    }
    output.push_str("\n  \x1b[2m↑↓ 选择 · Enter 确认 · y 允许 · n 拒绝\x1b[0m");
    output
}

/// 渲染已经完成选择的权限事件。
///
/// 参数:
/// - `request`: 已处理权限请求
/// - `decision`: 用户权限决定
///
/// 返回:
/// - 可写入 TUI transcript 的 ANSI 文本块
pub(crate) fn render_permission_event(
    request: &PermissionRequest,
    decision: &PermissionDecision,
) -> String {
    let mut output = render_request_body(request);
    match decision {
        PermissionDecision::Allow => {
            output.push_str("\n  \x1b[32m已允许一次\x1b[0m");
        }
        PermissionDecision::Deny { reply } => {
            output.push_str("\n  \x1b[31m已拒绝\x1b[0m");
            if let Some(reply) = reply.as_deref().filter(|value| !value.trim().is_empty()) {
                output.push_str("\n  \x1b[2m回复: \x1b[0m");
                output.push_str(reply.trim());
            }
        }
    }
    output
}

/// 渲染权限请求的语义化主体（不含完整 patch 正文）。
///
/// 参数:
/// - `request`: 权限请求
///
/// 返回:
/// - 不含选择项和最终决定的 ANSI 文本块
fn render_request_body(request: &PermissionRequest) -> String {
    let presentation = PermissionPresentation::from_request(request);
    let mut output = format!(
        "\x1b[1;33m需要权限\x1b[0m\n  \x1b[1m{}\x1b[0m  {}",
        presentation.action, presentation.summary
    );
    for detail in presentation.details {
        // patch / content 已有独立 diff 视图，权限确认处不再重复铺全文。
        if detail.label == "变更" || detail.label == "内容" {
            continue;
        }
        output.push_str(&format!("\n\n  \x1b[2m{}\x1b[0m\n", detail.label));
        for line in detail.value.lines() {
            output.push_str("    ");
            output.push_str(line);
            output.push('\n');
        }
        output.pop();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证终端权限提示使用语义化字段和可导航选择项。
    #[test]
    fn permission_prompt_does_not_render_raw_json() {
        let request = PermissionRequest {
            id: "id".to_string(),
            session_id: "session".to_string(),
            tool: "edit_file".to_string(),
            arguments: r#"{"path":"src/main.rs","content":"fn main() {}"}"#.to_string(),
        };

        let output = render_permission_prompt(&request, PermissionChoice::Allow);

        assert!(output.contains("src/main.rs"));
        assert!(!output.contains(r#"{"path""#));
        assert!(output.contains("❯"));
        assert!(output.contains("允许一次"));
        // 内容详情已折叠，避免与 diff 视图重复。
        assert!(!output.contains("fn main() {}"));
    }

    #[test]
    fn permission_choice_moves_with_wrap_limits() {
        assert_eq!(PermissionChoice::Allow.next(), PermissionChoice::Deny);
        assert_eq!(PermissionChoice::DenyWithReply.next(), PermissionChoice::DenyWithReply);
        assert_eq!(PermissionChoice::Allow.prev(), PermissionChoice::Allow);
        assert_eq!(PermissionChoice::Deny.prev(), PermissionChoice::Allow);
    }
}
