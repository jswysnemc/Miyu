use crate::render::command_output::render_command_block_with_action;
use crate::render::edit_diff::render_edit_file_diff;
use crate::render::tool_event_line::{tool_event_label, tool_event_text};
use crate::render::ToolCallDisplayMode;
use crate::tools::subagent_timeline::SubagentTimelineEntry;
use serde_json::Value;
use std::hash::{Hash, Hasher};

const RESULT_LIMIT: usize = 4_000;

/// 子智能体内部的可见进度单元。
#[derive(Clone, Debug, Eq, PartialEq)]
enum SubagentPart {
    Reasoning(String),
    ToolCall {
        name: String,
        arguments: String,
    },
    ToolResult {
        name: String,
        ok: bool,
        output: String,
    },
    Progress(String),
}

/// 可持续更新的子智能体 TUI 单元。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SubagentCell {
    arguments: String,
    subagent_id: Option<String>,
    parts: Vec<SubagentPart>,
    outcome: Option<(bool, String)>,
}

impl SubagentCell {
    /// 创建正在执行的子智能体单元。
    ///
    /// 参数:
    /// - `arguments`: subagent 工具参数
    ///
    /// 返回:
    /// - 空时间线单元
    pub(crate) fn new(arguments: String) -> Self {
        Self {
            arguments,
            subagent_id: None,
            parts: Vec::new(),
            outcome: None,
        }
    }

    /// 记录一条子智能体进度。
    ///
    /// 参数:
    /// - `message`: 带内部类型前缀的进度文本
    ///
    /// 返回:
    /// - 无
    pub(crate) fn push_progress(&mut self, message: String) {
        let part = if let Some(text) = message.strip_prefix("__subagent_reasoning__") {
            SubagentPart::Reasoning(text.to_string())
        } else if let Some(payload) = message.strip_prefix("__subtool_call__") {
            parse_subtool_call(payload).unwrap_or_else(|| SubagentPart::Progress(message.clone()))
        } else if let Some(payload) = message.strip_prefix("__subtool_result__") {
            parse_subtool_result(payload).unwrap_or_else(|| SubagentPart::Progress(message.clone()))
        } else if message == "__external_output__" {
            return;
        } else {
            SubagentPart::Progress(message)
        };
        self.parts.push(part);
    }

    /// 完成子智能体单元。
    ///
    /// 参数:
    /// - `ok`: 执行是否成功
    /// - `output`: 最终结果
    ///
    /// 返回:
    /// - 无
    pub(crate) fn finish(&mut self, ok: bool, output: String) {
        // 【TUI】【子智能体绑定】1. 后台启动结果只绑定 ID，终态由持久化时间线驱动
        if ok {
            if let Some((id, status)) = subagent_identity(&output) {
                self.subagent_id = Some(id);
                if status == "running" {
                    return;
                }
            }
        }
        self.outcome = Some((ok, output));
    }

    /// 判断子智能体是否仍在执行。
    ///
    /// 返回:
    /// - 尚未完成时返回 true
    pub(crate) fn is_active(&self) -> bool {
        self.outcome.is_none() && self.subagent_id.is_none()
    }

    /// 判断后台子智能体是否仍会产生新时间线。
    ///
    /// 返回:
    /// - 子智能体仍在执行时返回 true
    pub(crate) fn has_live_updates(&self) -> bool {
        self.subagent_id
            .as_deref()
            .and_then(|id| crate::tools::subagent_state::subagent_snapshot(id).ok())
            .is_some_and(|snapshot| snapshot.status == "running")
    }

    /// 返回用于判断 TUI 是否需要重绘的状态签名。
    ///
    /// 返回:
    /// - `(ID, 状态, 更新时间, 时间线哈希)`
    pub(crate) fn state_signature(&self) -> Option<(String, String, u64, u64)> {
        // 【TUI】【子智能体重绘】1. 同时纳入快照和时间线，避免同一秒内的流式更新被忽略
        let id = self.subagent_id.as_deref()?;
        let snapshot = crate::tools::subagent_state::subagent_snapshot(id).ok()?;
        let timeline = crate::tools::subagent_state::subagent_timeline(id).ok()?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(&timeline).ok()?.hash(&mut hasher);
        Some((
            id.to_string(),
            snapshot.status,
            snapshot.updated_at,
            hasher.finish(),
        ))
    }
}

/// 渲染子智能体思考、工具和结果。
///
/// 参数:
/// - `cell`: 子智能体时间线
/// - `mode`: 工具展示模式
///
/// 返回:
/// - 复用主智能体工具视觉语言的 ANSI 文本
pub(super) fn render(cell: &SubagentCell, mode: ToolCallDisplayMode) -> String {
    if mode == ToolCallDisplayMode::Hidden {
        return String::new();
    }
    // 【TUI】【子智能体渲染】1. 优先使用后台持久化快照，工具内进度仅作前台兼容
    let snapshot = cell
        .subagent_id
        .as_deref()
        .and_then(|id| crate::tools::subagent_state::subagent_snapshot(id).ok());
    let timeline = cell
        .subagent_id
        .as_deref()
        .and_then(|id| crate::tools::subagent_state::subagent_timeline(id).ok())
        .map(parts_from_timeline);
    // 【TUI】【子智能体渲染】2. 使用主智能体工具标题和状态样式
    let label = snapshot
        .as_ref()
        .map(|snapshot| format!("Subagent {}", snapshot.description))
        .unwrap_or_else(|| tool_event_label("subagent", Some(&cell.arguments)));
    let status = snapshot
        .as_ref()
        .map(|snapshot| match snapshot.status.as_str() {
            "completed" => "ok",
            "failed" | "cancelled" => "err",
            _ => "run",
        })
        .unwrap_or_else(|| match cell.outcome.as_ref() {
            Some((true, _)) => "ok",
            Some((false, _)) => "err",
            None => "run",
        });
    let mut output = tool_event_text(&label, status);
    let parts = timeline.as_deref().unwrap_or(&cell.parts);
    let visible_parts = if mode == ToolCallDisplayMode::Summary {
        parts
            .iter()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
    } else {
        parts.iter().collect::<Vec<_>>()
    };
    // 【TUI】【子智能体渲染】3. 按思考、工具调用和结果顺序绘制时间线
    for part in visible_parts {
        output.push('\n');
        output.push_str(&render_part(part, mode));
    }
    // 【TUI】【子智能体渲染】4. 完整模式在时间线后附加最终结果
    if mode == ToolCallDisplayMode::Full {
        let result = snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.result.as_deref().or(snapshot.error.as_deref()))
            .or_else(|| cell.outcome.as_ref().map(|(_, result)| result.as_str()));
        if let Some(result) = result {
            if !result.trim().is_empty() {
                output.push_str("\n\x1b[2m  └─ 结果\x1b[0m\n");
                output.push_str(&truncate_chars(result.trim(), RESULT_LIMIT));
            }
        }
    }
    output
}

/// 从 subagent 工具结果中读取子智能体 ID 和状态。
///
/// 参数:
/// - `output`: subagent 工具 JSON 输出
///
/// 返回:
/// - 可识别时返回 `(ID, 状态)`
fn subagent_identity(output: &str) -> Option<(String, String)> {
    let value = serde_json::from_str::<Value>(output).ok()?;
    let subagent = value.get("subagent")?;
    Some((
        subagent.get("id")?.as_str()?.to_string(),
        subagent.get("status")?.as_str()?.to_string(),
    ))
}

/// 将持久化子智能体时间线转换为 TUI 单元。
///
/// 参数:
/// - `entries`: 持久化时间线条目
///
/// 返回:
/// - 可使用主智能体视觉语言渲染的单元
fn parts_from_timeline(entries: Vec<SubagentTimelineEntry>) -> Vec<SubagentPart> {
    entries
        .into_iter()
        .map(|entry| match entry {
            SubagentTimelineEntry::Reasoning { text } => SubagentPart::Reasoning(text),
            SubagentTimelineEntry::Text { text } => SubagentPart::Progress(text),
            SubagentTimelineEntry::Tool {
                name,
                args_preview,
                ok,
                output_preview,
                ..
            } => match ok {
                Some(ok) => SubagentPart::ToolResult {
                    name,
                    ok,
                    output: output_preview.unwrap_or_default(),
                },
                None => SubagentPart::ToolCall {
                    name,
                    arguments: args_preview,
                },
            },
        })
        .collect()
}

/// 渲染单个子智能体时间线单元。
///
/// 参数:
/// - `part`: 思考、工具或进度单元
/// - `mode`: 工具展示模式
///
/// 返回:
/// - ANSI 格式的子时间线文本
fn render_part(part: &SubagentPart, mode: ToolCallDisplayMode) -> String {
    match part {
        SubagentPart::Reasoning(text) => format!("\x1b[2m\x1b[36m  ├─ {text}\x1b[0m"),
        SubagentPart::Progress(text) => format!("\x1b[2m  ├─ {text}\x1b[0m"),
        SubagentPart::ToolCall { name, arguments } => {
            if mode == ToolCallDisplayMode::Full && name == "edit_file" {
                return render_edit_file_diff(arguments).unwrap_or_else(|| {
                    tool_event_text(&tool_event_label(name, Some(arguments)), "run")
                });
            }
            if mode == ToolCallDisplayMode::Full && name == "run_command" {
                return render_command_block_with_action(
                    arguments,
                    &tool_event_label(name, Some(arguments)),
                )
                .trim_end()
                .to_string();
            }
            format!(
                "\x1b[2m  ├─ {}\x1b[0m",
                tool_event_text(&tool_event_label(name, Some(arguments)), "run")
            )
        }
        SubagentPart::ToolResult { name, ok, output } => {
            let status = if *ok { "ok" } else { "err" };
            let mut rendered = format!(
                "\x1b[2m  └─ {}\x1b[0m",
                tool_event_text(&tool_event_label(name, None), status)
            );
            if mode == ToolCallDisplayMode::Full && !output.trim().is_empty() {
                rendered.push_str("\n");
                rendered.push_str(&truncate_chars(output.trim(), 1_200));
            }
            rendered
        }
    }
}

/// 解析子工具调用进度。
///
/// 参数:
/// - `payload`: `{name, args}` JSON 文本
///
/// 返回:
/// - 可识别时返回工具调用单元
fn parse_subtool_call(payload: &str) -> Option<SubagentPart> {
    let value = serde_json::from_str::<Value>(payload).ok()?;
    Some(SubagentPart::ToolCall {
        name: value.get("name")?.as_str()?.to_string(),
        arguments: value
            .get("args")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

/// 解析子工具结果进度。
///
/// 参数:
/// - `payload`: `{name, ok, output}` JSON 文本
///
/// 返回:
/// - 可识别时返回工具结果单元
fn parse_subtool_result(payload: &str) -> Option<SubagentPart> {
    let value = serde_json::from_str::<Value>(payload).ok()?;
    Some(SubagentPart::ToolResult {
        name: value.get("name")?.as_str()?.to_string(),
        ok: value.get("ok").and_then(Value::as_bool).unwrap_or(true),
        output: value
            .get("output")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

/// 按字符数限制子智能体输出。
///
/// 参数:
/// - `text`: 原始文本
/// - `limit`: 最大字符数
///
/// 返回:
/// - 限制后的文本
fn truncate_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut output = text.chars().take(limit).collect::<String>();
    output.push_str("\n...");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_reasoning_and_subtool_timeline() {
        let mut cell = SubagentCell::new(r#"{"description":"检查项目"}"#.to_string());
        cell.push_progress("__subagent_reasoning__先检查文件".to_string());
        cell.push_progress(
            r#"__subtool_call__{"name":"read_file","args":"{\"path\":\"README.md\"}"}"#.to_string(),
        );
        cell.push_progress(
            r#"__subtool_result__{"name":"read_file","ok":true,"output":"done"}"#.to_string(),
        );

        let rendered = render(&cell, ToolCallDisplayMode::Full);

        assert!(rendered.contains("先检查文件"));
        assert!(rendered.contains("README.md"));
        assert!(rendered.contains("done"));
    }
}
