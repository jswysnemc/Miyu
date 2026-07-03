use crate::i18n::text as t;
use crate::llm::ToolCallStreamProgress;
use crate::render::style::TOOL_BULLET;
use anyhow::Result;
use serde_json::Value;
use std::io::{self, Write};

/// 生成工具参数接收进度文本。
///
/// 参数:
/// - `name`: 工具展示名称
/// - `progress`: 工具调用参数流式进度
///
/// 返回:
/// - 可直接写入终端的单行进度文本
pub(crate) fn tool_call_progress_text(name: &str, progress: &ToolCallStreamProgress) -> String {
    let size = format_bytes(progress.arguments_bytes);
    format!(
        "{TOOL_BULLET} {}: {name} {} · {size} · {} {}",
        t("tool", "工具"),
        t("receiving arguments", "接收参数"),
        progress.arguments_chars,
        t("chars", "字符")
    )
}

/// 写入工具调用参数预览。
///
/// 参数:
/// - `stdout`: 标准输出句柄
/// - `label`: 参数块标签
/// - `name`: 工具名称
/// - `arguments`: 工具参数 JSON
///
/// 返回:
/// - 是否已写入专用预览
pub(crate) fn write_tool_call_preview(
    stdout: &mut io::Stdout,
    label: &str,
    name: &str,
    arguments: &str,
) -> Result<bool> {
    if name != "write_file" {
        return Ok(false);
    }
    let Some(summary) = write_file_summary(arguments) else {
        return Ok(false);
    };
    writeln!(stdout, "\x1b[2m{TOOL_BULLET} {label}:\x1b[0m")?;
    writeln!(stdout, "\x1b[2m  path: {}\x1b[0m", summary.path)?;
    writeln!(
        stdout,
        "\x1b[2m  content: {} · {} {} · {} {}\x1b[0m",
        format_bytes(summary.bytes),
        summary.chars,
        t("chars", "字符"),
        summary.lines,
        t("lines", "行")
    )?;
    Ok(true)
}

struct WriteFileSummary {
    path: String,
    chars: usize,
    bytes: usize,
    lines: usize,
}

/// 解析写文件工具参数摘要。
///
/// 参数:
/// - `arguments`: 工具参数 JSON
///
/// 返回:
/// - 写文件参数摘要，参数不完整时返回空
fn write_file_summary(arguments: &str) -> Option<WriteFileSummary> {
    let value = serde_json::from_str::<Value>(arguments).ok()?;
    let path = value.get("path").and_then(Value::as_str)?.to_string();
    let content = value.get("content").and_then(Value::as_str)?;
    Some(WriteFileSummary {
        path,
        chars: content.chars().count(),
        bytes: content.len(),
        lines: content
            .matches('\n')
            .count()
            .saturating_add((!content.is_empty()) as usize),
    })
}

/// 格式化字节大小。
///
/// 参数:
/// - `bytes`: 字节数
///
/// 返回:
/// - 适合终端展示的大小文本
fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;
    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_text_contains_size_and_chars() {
        let progress = ToolCallStreamProgress {
            index: 0,
            name: Some("write_file".to_string()),
            arguments_chars: 2048,
            arguments_bytes: 4096,
        };

        let output = tool_call_progress_text("write_file", &progress);

        assert!(output.contains("write_file"));
        assert!(output.contains("4.0 KB"));
        assert!(output.contains("2048"));
    }

    #[test]
    fn write_file_summary_does_not_include_content_body() {
        let summary = write_file_summary(r#"{"path":"a.txt","content":"hello\nworld"}"#).unwrap();

        assert_eq!(summary.path, "a.txt");
        assert_eq!(summary.chars, 11);
        assert_eq!(summary.bytes, 11);
        assert_eq!(summary.lines, 2);
    }
}
