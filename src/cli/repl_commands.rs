const REPL_COMMANDS: &[&str] = &[
    "/providers",
    "/config",
    "/ps",
    "/thinking",
    "/plan",
    "/yolo",
    "/undo",
    "/reset",
    "/help",
    "/exit",
];

/// 返回 REPL 支持的斜杠菜单。
///
/// 返回:
/// - 斜杠菜单列表
pub(super) fn repl_commands() -> &'static [&'static str] {
    REPL_COMMANDS
}

/// 根据当前输入生成斜杠菜单补全建议。
///
/// 参数:
/// - `input`: 当前输入内容
///
/// 返回:
/// - 可补全的斜杠菜单
pub(super) fn repl_command_suggestions(input: &str) -> Vec<&'static str> {
    if !input.starts_with('/') {
        return Vec::new();
    }
    repl_commands()
        .iter()
        .copied()
        .filter(|command| command.starts_with(input))
        .collect()
}

/// 返回唯一匹配的斜杠菜单补全文本。
///
/// 参数:
/// - `input`: 当前输入内容
///
/// 返回:
/// - 唯一补全结果
pub(super) fn complete_repl_command(input: &str) -> Option<&'static str> {
    let suggestions = repl_command_suggestions(input);
    if suggestions.len() == 1 {
        suggestions.first().copied()
    } else {
        None
    }
}

/// 提取斜杠菜单后面的参数文本。
///
/// 参数:
/// - `input`: 当前输入内容
/// - `command`: 斜杠菜单名称
///
/// 返回:
/// - 匹配时返回参数文本
pub(super) fn repl_command_rest<'a>(input: &'a str, command: &str) -> Option<&'a str> {
    let input = input.trim();
    if input.eq_ignore_ascii_case(command) {
        return Some("");
    }
    let rest = input.get(command.len()..)?;
    if input[..command.len()].eq_ignore_ascii_case(command)
        && rest.chars().next().is_some_and(char::is_whitespace)
    {
        return Some(rest.trim_start());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_is_a_repl_command() {
        assert!(repl_commands().contains(&"/reset"));
    }

    #[test]
    fn repl_commands_include_recent_management_entries() {
        assert!(repl_commands().contains(&"/thinking"));
        assert!(repl_commands().contains(&"/ps"));
        assert!(!repl_commands().contains(&"/commands"));
        assert!(!repl_commands().contains(&"/clipb"));
        assert!(!repl_commands().contains(&"/set"));
    }

    #[test]
    fn command_rest_requires_boundary() {
        assert_eq!(
            repl_command_rest("/thinking high", "/thinking"),
            Some("high")
        );
        assert_eq!(repl_command_rest("/think", "/thinking"), None);
    }

    #[test]
    fn ps_command_completes_background_manager() {
        assert_eq!(complete_repl_command("/ps"), Some("/ps"));
    }
}
