use crate::agent::AgentEvent;
use crate::i18n::text as t;
use crate::llm::ChatStreamKind;

/// 单轮请求的用户可见工作状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkStatus {
    WaitingResponse,
    Thinking,
    Working,
}

impl WorkStatus {
    /// 根据 Agent 事件计算下一工作状态。
    ///
    /// 参数:
    /// - `event`: 当前 Agent 事件
    ///
    /// 返回:
    /// - 需要更新时返回新状态
    pub(crate) fn from_agent_event(event: &AgentEvent) -> Option<Self> {
        match event {
            AgentEvent::Chunk(chunk) if chunk.kind == ChatStreamKind::Reasoning => {
                Some(Self::Thinking)
            }
            AgentEvent::Chunk(_)
            | AgentEvent::ToolCall { .. }
            | AgentEvent::ToolCallProgress(_)
            | AgentEvent::ToolResult { .. }
            | AgentEvent::ToolProgress { .. }
            | AgentEvent::CompactionStarted { .. }
            | AgentEvent::CompactionFinished { .. } => Some(Self::Working),
            AgentEvent::FlushContent | AgentEvent::ExternalOutput => None,
        }
    }

    /// 返回当前语言下的状态名称。
    ///
    /// 返回:
    /// - 工作状态文本
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::WaitingResponse => t("waiting for response", "等待响应"),
            Self::Thinking => t("thinking", "思考"),
            Self::Working => t("working", "工作"),
        }
    }

    /// 渲染适合历史区展示的状态行。
    ///
    /// 返回:
    /// - 带 ANSI 样式的状态行
    pub(crate) fn render_line(self) -> String {
        format!("\x1b[2m\x1b[36m• {}\x1b[0m", self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::ChatStreamChunk;

    #[test]
    fn reasoning_and_content_map_to_distinct_states() {
        let reasoning = AgentEvent::Chunk(ChatStreamChunk {
            kind: ChatStreamKind::Reasoning,
            text: "inspect".to_string(),
        });
        let content = AgentEvent::Chunk(ChatStreamChunk {
            kind: ChatStreamKind::Content,
            text: "answer".to_string(),
        });

        assert_eq!(
            WorkStatus::from_agent_event(&reasoning),
            Some(WorkStatus::Thinking)
        );
        assert_eq!(
            WorkStatus::from_agent_event(&content),
            Some(WorkStatus::Working)
        );
    }
}
