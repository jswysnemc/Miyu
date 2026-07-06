use crate::state::turns::{Turn, TurnStatus};

const SUMMARY_TEMPLATE: &str = r#"Output exactly this Markdown structure and keep the section order unchanged:
---
## Goal
- [single-sentence task summary]

## Constraints & Preferences
- [user constraints, preferences, specs, or "(none)"]

## Progress
### Done
- [completed work or "(none)"]

### In Progress
- [current work or "(none)"]

### Blocked
- [blockers or "(none)"]

## Key Decisions
- [decision and why, or "(none)"]

## Next Steps
- [ordered next actions or "(none)"]

## Critical Context
- [important technical facts, errors, open questions, or "(none)"]

## Relevant Files
- [file or directory path: why it matters, or "(none)"]
---

Rules:
- Keep every section, even when empty.
- Use terse bullets, not prose paragraphs.
- Preserve exact file paths, commands, error strings, and identifiers when known.
- Do not mention the summary process or that context was compacted."#;

const TOOL_REPORT_MAX_CHARS: usize = 2_000;

/// 构造压缩摘要提示词。
///
/// 参数:
/// - `previous_summary`: 旧压缩摘要
/// - `turns`: 本次需要压缩的轮次
///
/// 返回:
/// - 发送给模型的摘要提示词
pub fn build_summary_prompt(previous_summary: Option<&str>, turns: &[Turn]) -> String {
    let anchor = match previous_summary.map(str::trim).filter(|value| !value.is_empty()) {
        Some(summary) => format!(
            "Update the anchored summary below using the conversation history above.\nPreserve still-true details, remove stale details, and merge in the new facts.\n\n<previous-summary>\n{summary}\n</previous-summary>"
        ),
        None => "Create a new anchored summary from the conversation history above.".to_string(),
    };
    let history = format_turns_for_summary(turns);
    format!(
        "{anchor}\n\n{SUMMARY_TEMPLATE}\n\n<conversation-history>\n{history}\n</conversation-history>"
    )
}

/// 构造注入对话上下文的摘要消息。
///
/// 参数:
/// - `summary`: 会话压缩摘要
///
/// 返回:
/// - 系统上下文消息
pub fn summary_context_message(summary: &str) -> String {
    format!(
        "<conversation-summary>\nThe following summary preserves earlier conversation context that is no longer present as raw messages.\n\n{}\n</conversation-summary>",
        summary.trim()
    )
}

/// 格式化轮次为摘要输入。
///
/// 参数:
/// - `turns`: 对话轮次
///
/// 返回:
/// - 可读的轮次文本
fn format_turns_for_summary(turns: &[Turn]) -> String {
    turns
        .iter()
        .map(format_turn_for_summary)
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// 格式化单个轮次为摘要输入。
///
/// 参数:
/// - `turn`: 对话轮次
///
/// 返回:
/// - 可读的轮次文本
fn format_turn_for_summary(turn: &Turn) -> String {
    let mut parts = vec![
        format!(
            "<turn id=\"{}\" status=\"{}\">",
            turn.turn_id,
            status_name(turn.status)
        ),
        format!("<user>\n{}\n</user>", turn.user_content.trim()),
    ];
    if let Some(reasoning) = turn
        .assistant_reasoning
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(format!(
            "<assistant-reasoning>\n{reasoning}\n</assistant-reasoning>"
        ));
    }
    parts.push(format!(
        "<assistant>\n{}\n</assistant>",
        turn.assistant_content.trim()
    ));
    if !turn.tool_reports.is_empty() {
        parts.push(format!(
            "<tool-reports>\n{}\n</tool-reports>",
            format_tool_reports(&turn.tool_reports)
        ));
    }
    parts.push("</turn>".to_string());
    parts.join("\n")
}

/// 格式化工具报告。
///
/// 参数:
/// - `reports`: 工具报告列表
///
/// 返回:
/// - 截断后的报告文本
fn format_tool_reports(reports: &[String]) -> String {
    reports
        .iter()
        .map(|report| truncate_chars(report.trim(), TOOL_REPORT_MAX_CHARS))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// 截断字符文本。
///
/// 参数:
/// - `value`: 原始文本
/// - `max_chars`: 最大字符数
///
/// 返回:
/// - 截断后的文本
fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut iter = value.chars();
    let truncated = iter.by_ref().take(max_chars).collect::<String>();
    if iter.next().is_some() {
        format!("{truncated}\n[truncated]")
    } else {
        truncated
    }
}

/// 返回轮次状态名称。
///
/// 参数:
/// - `status`: 轮次状态
///
/// 返回:
/// - 状态名称
fn status_name(status: TurnStatus) -> &'static str {
    match status {
        TurnStatus::Running => "running",
        TurnStatus::Completed => "completed",
        TurnStatus::Interrupted => "interrupted",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::turns::pending_placeholder;

    fn test_turn() -> Turn {
        Turn {
            turn_id: "turn_1".to_string(),
            seq: 1,
            user_content: "implement feature".to_string(),
            user_timestamp: "2026-01-01T00:00:00Z".to_string(),
            assistant_content: "implemented src/main.rs".to_string(),
            assistant_reasoning: None,
            assistant_timestamp: Some("2026-01-01T00:00:01Z".to_string()),
            status: TurnStatus::Completed,
            tool_reports: Vec::new(),
        }
    }

    #[test]
    fn prompt_includes_previous_summary_and_turns() {
        let prompt = build_summary_prompt(Some("old summary"), &[test_turn()]);

        assert!(prompt.contains("<previous-summary>"));
        assert!(prompt.contains("old summary"));
        assert!(prompt.contains("implement feature"));
        assert!(prompt.contains("implemented src/main.rs"));
    }

    #[test]
    fn summary_context_uses_stable_wrapper() {
        let message = summary_context_message("summary");

        assert!(message.starts_with("<conversation-summary>"));
        assert!(message.contains("summary"));
    }

    #[test]
    fn running_placeholder_can_be_formatted() {
        let mut turn = test_turn();
        turn.status = TurnStatus::Running;
        turn.assistant_content = pending_placeholder().to_string();

        let prompt = build_summary_prompt(None, &[turn]);

        assert!(prompt.contains("status=\"running\""));
    }
}
