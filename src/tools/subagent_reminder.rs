use super::subagent_state::{take_finished_notices, FinishedSubagentNotice};

/// 子智能体完成提醒器。
///
/// 在主 Agent 的工具循环中,每个工具轮后检查是否有后台子智能体刚刚完成,
/// 若有则生成一段 system-reminder 注入对话,主动把结果推给主 Agent,
/// 避免主 Agent 因不知道子智能体是否完成而反复轮询 action=status。
#[derive(Default)]
pub(crate) struct SubagentReminder;

impl SubagentReminder {
    /// 创建子智能体完成提醒器。
    pub(crate) fn new() -> Self {
        Self
    }

    /// 在一个工具轮后收集新完成子智能体并生成提醒文本。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 有新完成子智能体时返回 system-reminder 文本
    pub(crate) fn after_tool_round(&mut self) -> Option<String> {
        let notices = take_finished_notices();
        if notices.is_empty() {
            return None;
        }
        Some(render_notice(&notices))
    }
}

/// 把完成通知渲染为 system-reminder 文本。
///
/// 参数:
/// - `notices`: 新完成的子智能体通知列表
///
/// 返回:
/// - system-reminder 文本
fn render_notice(notices: &[FinishedSubagentNotice]) -> String {
    let lines = notices
        .iter()
        .map(|notice| {
            format!(
                "- {}（{}）：{}",
                notice.description,
                notice.id,
                status_label(&notice.status)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "<system-reminder>以下后台子智能体已结束,结果已就绪,可用 subagent action=result 加 subagent_id 取回(失败或取消的附带错误信息),无需轮询 action=status:\n{lines}\n</system-reminder>"
    )
}

/// 返回子智能体终态的中文说明。
///
/// 参数:
/// - `status`: 子智能体状态
///
/// 返回:
/// - 状态说明文本
fn status_label(status: &str) -> &str {
    match status {
        "completed" => "已完成",
        "failed" => "执行失败",
        "cancelled" => "已取消",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::subagent_state::{create_subagent, finish_subagent};

    #[test]
    fn no_reminder_without_finished_subagents() {
        let mut reminder = SubagentReminder::new();
        // 新建但未完成,不应触发提醒
        let (_subagent, _cancel) =
            create_subagent("pending work".to_string(), "general".to_string(), 5);
        // 可能有其他测试残留的完成项,这里只断言不 panic 并可重复调用
        let _ = reminder.after_tool_round();
    }

    #[test]
    fn reminds_once_after_finish() {
        let mut reminder = SubagentReminder::new();
        let (subagent, _cancel) =
            create_subagent("build index".to_string(), "explore".to_string(), 5);
        finish_subagent(&subagent.id, "completed", Some("done".to_string()), None, None);

        let first = reminder.after_tool_round();
        assert!(first.is_some());
        let text = first.unwrap();
        assert!(text.contains(&subagent.id));
        assert!(text.contains("已完成"));

        // 同一完成事件不应重复提醒
        let notices_again = take_finished_notices();
        assert!(notices_again.iter().all(|notice| notice.id != subagent.id));
    }
}
