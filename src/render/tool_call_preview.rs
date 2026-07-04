use crate::i18n::text as t;
use crate::render::style::TOOL_BULLET;

/// 生成工具状态文本。
///
/// 参数:
/// - `name`: 工具展示名称
/// - `status`: 工具状态
///
/// 返回:
/// - 可直接写入终端的单行状态文本
pub(crate) fn tool_call_status_text(name: &str, status: &str) -> String {
    format!("{TOOL_BULLET} {}: {name} {status}", t("tool", "工具"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_text_uses_compact_state() {
        let output = tool_call_status_text("write_file", "arg");

        assert!(output.contains("write_file"));
        assert!(output.ends_with("arg"));
        assert!(!output.contains("receiving"));
    }
}
