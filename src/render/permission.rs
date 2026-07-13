use crate::permission::{PermissionDecision, PermissionPresentation, PermissionRequest};

/// 渲染终端中的权限请求详情和选择项。
///
/// 参数:
/// - `request`: 待处理权限请求
///
/// 返回:
/// - Codex 风格的 ANSI 文本块
pub(crate) fn render_permission_prompt(request: &PermissionRequest) -> String {
    let mut output = render_request_body(request);
    output.push_str("\n  \x1b[1m1.\x1b[0m 允许一次\n");
    output.push_str("  \x1b[1m2.\x1b[0m 拒绝\n");
    output.push_str("  \x1b[1m3.\x1b[0m 拒绝并告诉 Miyu 如何调整\n");
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

/// 渲染权限请求的语义化主体。
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

    /// 验证终端权限提示使用语义化字段和稳定选择项。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn permission_prompt_does_not_render_raw_json() {
        let request = PermissionRequest {
            id: "id".to_string(),
            session_id: "session".to_string(),
            tool: "edit_file".to_string(),
            arguments: r#"{"path":"src/main.rs","content":"fn main() {}"}"#.to_string(),
        };

        let output = render_permission_prompt(&request);

        assert!(output.contains("src/main.rs"));
        assert!(!output.contains(r#"{"path""#));
        assert!(output.contains("1."));
    }
}
