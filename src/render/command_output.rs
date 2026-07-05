use crate::i18n::text as t;
use crate::render::code_block::{highlight_code_line, render_code_footer, render_code_header};
use crate::render::style::TOOL_BULLET;
use anyhow::Result;
use serde_json::Value;
use std::io::{self, Write};
use std::path::Path;

/// 写入普通工具参数或输出块。
///
/// 参数:
/// - `stdout`: 标准输出句柄
/// - `label`: 输出块标签
/// - `payload`: 原始工具载荷
///
/// 返回:
/// - 写入是否成功
pub(crate) fn write_tool_payload(
    stdout: &mut io::Stdout,
    label: &str,
    payload: &str,
) -> Result<()> {
    let formatted = format_tool_payload(payload);
    writeln!(stdout, "\x1b[2m{TOOL_BULLET} {label}:\x1b[0m")?;
    for line in formatted.lines() {
        writeln!(stdout, "\x1b[2m  {line}\x1b[0m")?;
    }
    Ok(())
}

/// 写入带动作标题的命令调用块。
///
/// 参数:
/// - `stdout`: 标准输出句柄
/// - `arguments`: 工具调用参数
/// - `action`: 命令动作展示名
///
/// 返回:
/// - 写入是否成功
pub(crate) fn write_command_block_with_action(
    stdout: &mut io::Stdout,
    arguments: &str,
    action: &str,
) -> Result<()> {
    write!(
        stdout,
        "{}",
        render_command_block_with_action(arguments, action)
    )?;
    Ok(())
}

/// 写入编辑文件 diff 视图。
///
/// 参数:
/// - `stdout`: 标准输出句柄
/// - `arguments`: `edit_file` 工具参数
///
/// 返回:
/// - 是否成功渲染 diff 视图
pub(crate) fn write_edit_file_diff_block(stdout: &mut io::Stdout, arguments: &str) -> Result<bool> {
    let Some(diff) = render_edit_file_diff(arguments) else {
        return Ok(false);
    };
    write!(stdout, "{diff}")?;
    Ok(true)
}

/// 写入整文件写入 diff 视图。
///
/// 参数:
/// - `stdout`: 标准输出句柄
/// - `arguments`: `write_file` 工具参数
///
/// 返回:
/// - 是否成功渲染 diff 视图
pub(crate) fn write_write_file_diff_block(
    stdout: &mut io::Stdout,
    arguments: &str,
) -> Result<bool> {
    let Some(diff) = render_write_file_diff(arguments) else {
        return Ok(false);
    };
    write!(stdout, "{diff}")?;
    Ok(true)
}

/// 渲染命令调用块。
///
/// 参数:
/// - `arguments`: 工具调用参数
///
/// 返回:
/// - 代码块风格的命令文本
#[cfg(test)]
fn render_command_block(arguments: &str) -> String {
    render_command_block_with_action(arguments, "")
}

/// 渲染带动作标题的命令调用块。
///
/// 参数:
/// - `arguments`: 工具调用参数
/// - `action`: 命令动作展示名
///
/// 返回:
/// - 代码块风格的命令文本
fn render_command_block_with_action(arguments: &str, action: &str) -> String {
    let parsed = serde_json::from_str::<Value>(arguments).ok();
    let command = parsed
        .as_ref()
        .and_then(|value| value.get("command"))
        .and_then(Value::as_str)
        .unwrap_or(arguments)
        .trim();
    let lines = shell_command_lines(command);
    let header = if action.trim().is_empty() {
        format!("{TOOL_BULLET} command")
    } else {
        format!("{TOOL_BULLET} {} command", action.trim())
    };
    let mut output = render_code_header(&header);
    for line in &lines {
        output.push_str(&highlight_code_line("sh", line));
        output.push('\n');
    }
    output.push_str(&render_code_footer(&lines));
    output
}

/// 渲染编辑文件 diff 视图。
///
/// 参数:
/// - `arguments`: `edit_file` 工具参数
///
/// 返回:
/// - 代码块风格 diff 文本
fn render_edit_file_diff(arguments: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(arguments).ok()?;
    let path = value.get("path").and_then(Value::as_str)?;
    let start_line = value.get("start_line").and_then(Value::as_u64)? as usize;
    let end_line = value.get("end_line").and_then(Value::as_u64)? as usize;
    let replacement = value.get("replacement").and_then(Value::as_str)?;
    if start_line == 0 || end_line == 0 || start_line > end_line {
        return None;
    }
    let original = std::fs::read_to_string(Path::new(path)).ok()?;
    let old_lines = original.lines().map(str::to_string).collect::<Vec<_>>();
    if start_line > old_lines.len() || end_line > old_lines.len() {
        return None;
    }
    let replacement = replacement.replace("\r\n", "\n").replace('\r', "\n");
    let new_lines = if replacement.is_empty() {
        Vec::new()
    } else {
        replacement.lines().map(str::to_string).collect::<Vec<_>>()
    };
    let removed = old_lines[start_line - 1..end_line].to_vec();
    Some(render_codex_edit_diff(
        "Edited", path, start_line, &old_lines, &removed, &new_lines,
    ))
}

/// 渲染整文件写入 diff 视图。
///
/// 参数:
/// - `arguments`: `write_file` 工具参数
///
/// 返回:
/// - Codex 风格 diff 文本
fn render_write_file_diff(arguments: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(arguments).ok()?;
    let path = value.get("path").and_then(Value::as_str)?;
    let content = value.get("content").and_then(Value::as_str)?;
    let old_lines = std::fs::read_to_string(Path::new(path))
        .ok()
        .map(|text| text.lines().map(str::to_string).collect::<Vec<_>>())
        .unwrap_or_default();
    let content = content.replace("\r\n", "\n").replace('\r', "\n");
    let new_lines = if content.is_empty() {
        Vec::new()
    } else {
        content.lines().map(str::to_string).collect::<Vec<_>>()
    };
    let removed = old_lines.clone();
    Some(render_codex_edit_diff(
        "Writed", path, 1, &old_lines, &removed, &new_lines,
    ))
}

/// 渲染 Codex 风格编辑 diff。
///
/// 参数:
/// - `action`: 操作名称
/// - `path`: 文件路径
/// - `start_line`: 变更起始行号
/// - `old_lines`: 文件原始行
/// - `removed`: 被替换的旧行
/// - `added`: 新行
///
/// 返回:
/// - Codex 风格 diff 文本
fn render_codex_edit_diff(
    action: &str,
    path: &str,
    start_line: usize,
    old_lines: &[String],
    removed: &[String],
    added: &[String],
) -> String {
    let mut output = format!(
        "{TOOL_BULLET} {action} {path} ({} {})\n",
        style_added_count(added.len()),
        style_removed_count(removed.len())
    );
    let width = old_lines.len().max(start_line + added.len()).max(1);
    let number_width = width.to_string().len().max(3);
    if start_line > 2 {
        output.push_str(&style_context_line(&format!(
            "{:>number_width$}    ⋮",
            start_line - 1
        )));
        output.push('\n');
    }
    if start_line > 1 {
        output.push_str(&style_context_line(&format!(
            "{:>number_width$}    {}",
            start_line - 1,
            old_lines[start_line - 2]
        )));
        output.push('\n');
    }
    for (offset, line) in removed.iter().enumerate() {
        output.push_str(&style_removed_line(&format!(
            "{:>number_width$} -  {line}",
            start_line + offset
        )));
        output.push('\n');
    }
    for (offset, line) in added.iter().enumerate() {
        output.push_str(&style_added_line(&format!(
            "{:>number_width$} +  {line}",
            start_line + offset
        )));
        output.push('\n');
    }
    let after_line = start_line + removed.len();
    if after_line <= old_lines.len() {
        output.push_str(&style_context_line(&format!(
            "{:>number_width$}    {}",
            after_line,
            old_lines[after_line - 1]
        )));
        output.push('\n');
    }
    if after_line < old_lines.len() {
        output.push_str(&style_context_line(&format!(
            "{:>number_width$}    ⋮",
            after_line + 1
        )));
        output.push('\n');
    }
    output
}

/// 给 diff 上下文行添加样式。
///
/// 参数:
/// - `line`: 上下文行
///
/// 返回:
/// - 带 ANSI 样式的上下文行
fn style_context_line(line: &str) -> String {
    format!("\x1b[2m{line}\x1b[0m")
}

/// 给 diff 删除行添加样式。
///
/// 参数:
/// - `line`: 删除行
///
/// 返回:
/// - 带 ANSI 样式的删除行
fn style_removed_line(line: &str) -> String {
    format!("\x1b[48;5;52m\x1b[31m{line}\x1b[0m")
}

/// 给 diff 新增行添加样式。
///
/// 参数:
/// - `line`: 新增行
///
/// 返回:
/// - 带 ANSI 样式的新增行
fn style_added_line(line: &str) -> String {
    format!("\x1b[48;5;22m\x1b[32m{line}\x1b[0m")
}

/// 给新增行数添加样式。
///
/// 参数:
/// - `count`: 新增行数
///
/// 返回:
/// - 带 ANSI 样式的新增行数
fn style_added_count(count: usize) -> String {
    format!("\x1b[32m+{count}\x1b[0m")
}

/// 给删除行数添加样式。
///
/// 参数:
/// - `count`: 删除行数
///
/// 返回:
/// - 带 ANSI 样式的删除行数
fn style_removed_count(count: usize) -> String {
    format!("\x1b[31m-{count}\x1b[0m")
}

/// 生成命令代码块行。
///
/// 参数:
/// - `command`: 原始命令文本
///
/// 返回:
/// - 命令行列表
fn shell_command_lines(command: &str) -> Vec<String> {
    let mut lines = command.lines().map(str::to_string).collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// 按命令执行结果写入 stdout 和 stderr 块。
///
/// 参数:
/// - `stdout`: 标准输出句柄
/// - `output`: 命令执行结果 JSON
///
/// 返回:
/// - 写入是否成功
pub(crate) fn write_command_result_blocks(stdout: &mut io::Stdout, output: &str) -> Result<()> {
    let Some(result) = parse_command_result(output) else {
        return write_tool_payload(stdout, t("output", "输出"), output);
    };
    if !result.stdout.trim().is_empty() {
        write_output_block(stdout, t("output", "输出"), &result.stdout)?;
    }
    if !result.stderr.trim().is_empty() {
        let label = result
            .exit_code
            .map(|code| format!("err exit {code}"))
            .unwrap_or_else(|| "err".to_string());
        write_output_block(stdout, &label, &result.stderr)?;
    } else if !result.success {
        let label = result
            .exit_code
            .map(|code| format!("err exit {code}"))
            .unwrap_or_else(|| "err".to_string());
        write_output_block(
            stdout,
            &label,
            t(
                "command failed without stderr",
                "命令失败，但没有 stderr 输出",
            ),
        )?;
    }
    Ok(())
}

/// 写入命令失败摘要块。
///
/// 参数:
/// - `stdout`: 标准输出句柄
/// - `output`: 命令执行结果 JSON
///
/// 返回:
/// - 写入是否成功
pub(crate) fn write_command_error_block(stdout: &mut io::Stdout, output: &str) -> Result<()> {
    let Some(result) = parse_command_result(output) else {
        return write_output_block(stdout, "err", output);
    };
    if result.success {
        return Ok(());
    }
    let label = result
        .exit_code
        .map(|code| format!("err exit {code}"))
        .unwrap_or_else(|| "err".to_string());
    let message = if result.stderr.trim().is_empty() {
        result.stdout.as_str()
    } else {
        result.stderr.as_str()
    };
    write_output_block(stdout, &label, message)
}

/// 写入命令输出文本块。
///
/// 参数:
/// - `stdout`: 标准输出句柄
/// - `label`: 文本块标签
/// - `text`: 文本内容
///
/// 返回:
/// - 写入是否成功
fn write_output_block(stdout: &mut io::Stdout, label: &str, text: &str) -> Result<()> {
    write!(stdout, "{}", render_output_block(label, text))?;
    Ok(())
}

/// 渲染命令输出文本块。
///
/// 参数:
/// - `label`: 文本块标签
/// - `text`: 文本内容
///
/// 返回:
/// - 代码块风格的输出文本
fn render_output_block(label: &str, text: &str) -> String {
    let content = truncate_chars(text.trim(), 2400);
    let lines = output_block_lines(&content);
    let mut output = render_code_header(&format!("{TOOL_BULLET} {label}"));
    for line in &lines {
        output.push_str(line);
        output.push('\n');
    }
    output.push_str(&render_code_footer(&lines));
    output
}

/// 生成输出代码块行。
///
/// 参数:
/// - `content`: 已截断的输出文本
///
/// 返回:
/// - 输出行列表
fn output_block_lines(content: &str) -> Vec<String> {
    let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

struct CommandResult {
    success: bool,
    exit_code: Option<i64>,
    stdout: String,
    stderr: String,
}

/// 解析命令工具返回的 JSON 结果。
///
/// 参数:
/// - `output`: 原始 JSON 文本
///
/// 返回:
/// - 解析后的命令结果，解析失败时返回空
fn parse_command_result(output: &str) -> Option<CommandResult> {
    let value = serde_json::from_str::<Value>(output.trim()).ok()?;
    Some(CommandResult {
        success: value.get("success")?.as_bool()?,
        exit_code: value.get("exit_code").and_then(Value::as_i64),
        stdout: value
            .get("stdout")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        stderr: value
            .get("stderr")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

/// 格式化工具载荷并限制长度。
///
/// 参数:
/// - `payload`: 原始工具载荷
///
/// 返回:
/// - 格式化后的文本
fn format_tool_payload(payload: &str) -> String {
    let text = payload.trim();
    let formatted = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| text.to_string());
    truncate_chars(&formatted, 2400)
}

/// 按字符数量截断文本。
///
/// 参数:
/// - `text`: 原始文本
/// - `max_chars`: 最大字符数
///
/// 返回:
/// - 截断后的文本
fn truncate_chars(text: &str, max_chars: usize) -> String {
    let total = text.chars().count();
    if total <= max_chars {
        return text.to_string();
    }
    let omitted = total - max_chars;
    format!(
        "{}\n... {} {omitted} {} ...",
        text.chars().take(max_chars).collect::<String>(),
        t("truncated", "已截断"),
        t("chars", "字符")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_command_result_json() {
        let result = parse_command_result(
            r#"{"success":false,"exit_code":1,"stdout":"unused","stderr":"not found"}"#,
        )
        .unwrap();
        assert!(!result.success);
        assert_eq!(result.exit_code, Some(1));
        assert_eq!(result.stdout, "unused");
        assert_eq!(result.stderr, "not found");
    }

    #[test]
    fn renders_multiline_command_as_code_block() {
        let output = render_command_block(
            r#"{"command":"python3 - <<'PY'\nfrom pathlib import Path\nprint(Path('x').resolve())\nPY"}"#,
        );
        let plain = strip_ansi_for_test(&output);

        assert!(plain.contains("── • command "));
        assert!(plain.contains("python3 - <<'PY'"));
        assert!(!plain.contains("$ python3"));
        assert!(plain.contains("from pathlib import Path"));
        assert!(plain.contains("print(Path('x').resolve())"));
        assert!(plain.contains("\nPY\n"));
        assert!(!plain.contains(",-- command"));
        assert!(!plain.contains("`--"));
    }

    #[test]
    fn renders_command_block_with_action_header() {
        let output = render_command_block_with_action(r#"{"command":"date"}"#, "Run");
        let plain = strip_ansi_for_test(&output);

        assert!(plain.contains("── • Run command "));
        assert!(plain.contains("date"));
        assert!(!plain.contains("Run run"));
        assert!(!plain.contains("── • command "));
    }

    #[test]
    fn renders_background_command_block_with_distinct_header() {
        let output = render_command_block_with_action(r#"{"command":"sleep 1"}"#, "Background");
        let plain = strip_ansi_for_test(&output);

        assert!(plain.contains("── • Background command "));
        assert!(plain.contains("sleep 1"));
        assert!(!plain.contains("Run command"));
    }

    #[test]
    fn renders_edit_file_arguments_as_diff_block() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), "one\nold line\nthree\n").unwrap();
        let args = json!({
            "path": temp.path().to_string_lossy(),
            "start_line": 2,
            "end_line": 2,
            "replacement": "new line"
        })
        .to_string();

        let output = render_edit_file_diff(&args).unwrap();
        let plain = strip_ansi_for_test(&output);

        assert!(plain.contains("• Edited "));
        assert!(plain.contains("(+1 -1)"));
        assert!(output.contains("(\x1b[32m+1\x1b[0m \x1b[31m-1\x1b[0m)"));
        assert!(plain.contains("  1    one"));
        assert!(plain.contains("  2 -  old line"));
        assert!(plain.contains("  2 +  new line"));
        assert!(plain.contains("  3    three"));
        assert!(!plain.contains("--- "));
        assert!(!plain.contains("@@"));
        assert!(output.contains("\x1b[48;5;52m\x1b[31m"));
        assert!(output.contains("\x1b[48;5;22m\x1b[32m"));
    }

    #[test]
    fn renders_write_file_new_file_as_added_diff_block() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.txt");
        let args = json!({
            "path": path.to_string_lossy(),
            "content": "hello\nworld"
        })
        .to_string();

        let output = render_write_file_diff(&args).unwrap();
        let plain = strip_ansi_for_test(&output);

        assert!(plain.contains("• Writed "));
        assert!(plain.contains("(+2 -0)"));
        assert!(output.contains("(\x1b[32m+2\x1b[0m \x1b[31m-0\x1b[0m)"));
        assert!(plain.contains("  1 +  hello"));
        assert!(plain.contains("  2 +  world"));
        assert!(!plain.contains("content:"));
    }

    #[test]
    fn renders_write_file_overwrite_as_full_file_diff_block() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), "old\ntext\n").unwrap();
        let args = json!({
            "path": temp.path().to_string_lossy(),
            "content": "new\ntext"
        })
        .to_string();

        let output = render_write_file_diff(&args).unwrap();
        let plain = strip_ansi_for_test(&output);

        assert!(plain.contains("• Writed "));
        assert!(plain.contains("(+2 -2)"));
        assert!(output.contains("(\x1b[32m+2\x1b[0m \x1b[31m-2\x1b[0m)"));
        assert!(plain.contains("  1 -  old"));
        assert!(plain.contains("  2 -  text"));
        assert!(plain.contains("  1 +  new"));
        assert!(plain.contains("  2 +  text"));
    }

    #[test]
    fn renders_command_output_as_code_block() {
        let output = render_output_block("output", "sent to snemc@qq.com\n");
        let plain = strip_ansi_for_test(&output);

        assert!(plain.contains("── • output "));
        assert!(plain.contains("sent to snemc@qq.com"));
        assert!(!plain.contains(",-- output"));
        assert!(!plain.contains("`--"));
    }

    #[test]
    fn renders_command_error_output_as_code_block() {
        let output = render_output_block("err exit 1", "not found\n");
        let plain = strip_ansi_for_test(&output);

        assert!(plain.contains("── • err exit 1 "));
        assert!(plain.contains("not found"));
        assert!(!plain.contains(",-- err"));
        assert!(!plain.contains("`--"));
    }

    /// 去除 ANSI 转义序列，方便断言可见文本。
    ///
    /// 参数:
    /// - `text`: 原始终端文本
    ///
    /// 返回:
    /// - 去除样式后的文本
    fn strip_ansi_for_test(text: &str) -> String {
        let mut output = String::new();
        let mut escape = false;
        let mut csi = false;
        for ch in text.chars() {
            if ch == '\x1b' {
                escape = true;
                csi = false;
            } else if escape {
                if csi {
                    if (ch as u32) >= 0x40 && (ch as u32) <= 0x7e {
                        escape = false;
                    }
                } else if ch == '[' {
                    csi = true;
                } else if ch == '\\' || ch == 'm' {
                    escape = false;
                }
            } else {
                output.push(ch);
            }
        }
        output
    }
}
