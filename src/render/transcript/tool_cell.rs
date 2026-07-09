use crate::render::command_output::render_command_block_with_action;
use crate::render::tool_event_line::{tool_event_label, tool_event_text};
use crate::render::ToolCallDisplayMode;
use serde_json::Value;

/// 工具生命周期中的可重放 source 数据。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ToolCell {
    Call {
        name: String,
        arguments: String,
    },
    Result {
        name: String,
        ok: bool,
        output: String,
    },
    Progress {
        name: String,
        message: String,
    },
    CompactionStarted {
        turn_count: usize,
    },
    CompactionFinished {
        applied: bool,
    },
}

/// 渲染工具生命周期项。
///
/// 参数:
/// - `cell`: 工具 source 数据
/// - `mode`: 工具调用展示模式
///
/// 返回:
/// - ANSI 文本块
pub(crate) fn render(cell: &ToolCell, mode: ToolCallDisplayMode) -> String {
    match cell {
        ToolCell::Call { name, arguments } => render_call(name, arguments, mode),
        ToolCell::Result { name, ok, output } => render_result(name, *ok, output, mode),
        ToolCell::Progress { name, message } => render_progress(name, message, mode),
        ToolCell::CompactionStarted { turn_count } => {
            tool_event_text(&format!("compact context×{turn_count}"), "run")
        }
        ToolCell::CompactionFinished { applied } => {
            tool_event_text("compact context", if *applied { "ok" } else { "skip" })
        }
    }
}

/// 渲染尚未收到完整参数的工具调用状态。
///
/// 参数:
/// - `name`: 工具名称
/// - `arguments_preview`: 当前参数预览
/// - `mode`: 工具调用展示模式
///
/// 返回:
/// - 可重绘的临时工具状态
pub(crate) fn render_live_call(
    name: &str,
    arguments_preview: &str,
    mode: ToolCallDisplayMode,
) -> String {
    if mode == ToolCallDisplayMode::Hidden {
        return String::new();
    }
    let label = tool_event_label(name, Some(arguments_preview));
    tool_event_text(&label, "arg")
}

/// 渲染定稿的工具调用。
fn render_call(name: &str, arguments: &str, mode: ToolCallDisplayMode) -> String {
    if mode == ToolCallDisplayMode::Hidden {
        return String::new();
    }
    if name == "run_command" || name == "background_command" {
        let action = if name == "background_command" {
            "Background"
        } else {
            "Run"
        };
        return render_command_block_with_action(arguments, action)
            .trim_end()
            .to_string();
    }
    if mode == ToolCallDisplayMode::Summary {
        return tool_event_text(&tool_event_label(name, Some(arguments)), "run");
    }
    let label = tool_event_label(name, Some(arguments));
    format!(
        "{}\n{}",
        tool_event_text(&label, "run"),
        render_payload("args", arguments)
    )
}

/// 渲染定稿的工具结果。
fn render_result(name: &str, ok: bool, output: &str, mode: ToolCallDisplayMode) -> String {
    if mode == ToolCallDisplayMode::Hidden {
        return String::new();
    }
    if mode == ToolCallDisplayMode::Summary && ok && matches!(name, "run_command" | "edit_file") {
        return String::new();
    }
    let status = if ok { "ok" } else { "err" };
    let label = tool_event_label(name, None);
    if mode == ToolCallDisplayMode::Summary {
        return tool_event_text(&label, status);
    }
    format!(
        "{}\n{}",
        tool_event_text(&label, status),
        render_payload("output", output)
    )
}

/// 渲染可保留的工具进度。
fn render_progress(name: &str, message: &str, mode: ToolCallDisplayMode) -> String {
    if mode == ToolCallDisplayMode::Hidden || message.starts_with("__") {
        return String::new();
    }
    let label = tool_event_label(name, None);
    if message.trim().is_empty() {
        return tool_event_text(&label, "run");
    }
    format!(
        "{}\n\x1b[2m  {message}\x1b[0m",
        tool_event_text(&label, "run")
    )
}

/// 格式化并截断工具原始载荷。
fn render_payload(label: &str, payload: &str) -> String {
    let formatted = serde_json::from_str::<Value>(payload)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| payload.trim().to_string());
    let truncated = truncate_chars(&formatted, 2_400);
    let body = truncated
        .lines()
        .map(|line| format!("\x1b[2m  {line}\x1b[0m"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("\x1b[2m• {label}:\x1b[0m\n{body}")
}

/// 按字符限制工具载荷长度。
fn truncate_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut output = text.chars().take(limit).collect::<String>();
    output.push_str("\n…");
    output
}
