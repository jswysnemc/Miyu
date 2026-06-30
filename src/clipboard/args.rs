#[derive(Debug, Clone)]
pub struct ClipboardCliInput {
    pub message: String,
    pub clipb: bool,
}

/// 从普通 CLI 消息参数中提取剪贴板标志。
///
/// 参数:
/// - `parts`: 原始消息参数
/// - `clipb`: clap 已解析的剪贴板标志
///
/// 返回:
/// - 归一化后的消息和剪贴板标志
pub fn chat_input_from_parts(parts: Vec<String>, clipb: bool) -> ClipboardCliInput {
    let mut has_clipboard = clipb;
    let mut message_parts = parts;
    while message_parts
        .first()
        .map(|part| matches!(part.as_str(), "-c" | "--clipb"))
        .unwrap_or(false)
    {
        has_clipboard = true;
        message_parts.remove(0);
    }
    while message_parts
        .last()
        .map(|part| matches!(part.as_str(), "-c" | "--clipb"))
        .unwrap_or(false)
    {
        has_clipboard = true;
        message_parts.pop();
    }
    ClipboardCliInput {
        message: join_message(message_parts),
        clipb: has_clipboard,
    }
}

/// 组装聊天消息文本。
///
/// 参数:
/// - `parts`: 消息参数
///
/// 返回:
/// - 空格连接后的消息
fn join_message(parts: Vec<String>) -> String {
    parts.join(" ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_trailing_clipboard_flag() {
        let input = chat_input_from_parts(
            vec!["总结".to_string(), "这段".to_string(), "-c".to_string()],
            false,
        );
        assert!(input.clipb);
        assert_eq!(input.message, "总结 这段");
    }

    #[test]
    fn extracts_long_clipboard_flag() {
        let input = chat_input_from_parts(
            vec![
                "--clipb".to_string(),
                "总结".to_string(),
                "这段".to_string(),
            ],
            false,
        );
        assert!(input.clipb);
        assert_eq!(input.message, "总结 这段");
    }

    #[test]
    fn keeps_middle_clipboard_flag_literal() {
        let input = chat_input_from_parts(
            vec!["解释".to_string(), "-c".to_string(), "参数".to_string()],
            false,
        );
        assert!(!input.clipb);
        assert_eq!(input.message, "解释 -c 参数");
    }
}
