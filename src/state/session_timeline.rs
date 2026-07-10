use super::tool_history::load_tool_exchanges_for_turn;
use super::turns::TurnStatus;
use super::{StateStore, ToolCallStatus};
use anyhow::Result;
use serde::Serialize;

/// 会话时间线中的消息。
#[derive(Debug, Clone, Serialize)]
pub struct TimelineMessage {
    pub timestamp: String,
    pub content: String,
    pub reasoning: Option<String>,
}

/// 会话时间线中的工具调用。
#[derive(Debug, Clone, Serialize)]
pub struct TimelineToolEntry {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub status: String,
    pub output: String,
    pub ok: Option<bool>,
    pub error: Option<String>,
    pub result_ref: Option<String>,
    pub original_chars: Option<usize>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// 按轮次组织的会话时间线。
#[derive(Debug, Clone, Serialize)]
pub struct SessionTimelineTurn {
    pub turn_id: String,
    pub seq: i64,
    pub status: String,
    pub user: TimelineMessage,
    pub assistant: TimelineMessage,
    pub tools: Vec<TimelineToolEntry>,
}

impl StateStore {
    /// 读取最近会话轮次及其结构化工具历史。
    ///
    /// 参数:
    /// - `limit`: 最大轮次数量
    ///
    /// 返回:
    /// - 按对话顺序排列的会话时间线
    pub fn session_timeline(&self, limit: usize) -> Result<Vec<SessionTimelineTurn>> {
        let mut turns = self.conv_db.load_turns()?;
        let start = turns.len().saturating_sub(limit);
        let turns = turns.split_off(start);
        turns
            .into_iter()
            .map(|turn| {
                let exchanges =
                    load_tool_exchanges_for_turn(&self.conv_db, &self.session_id, &turn.turn_id)?;
                let tools = exchanges
                    .into_iter()
                    .map(|exchange| TimelineToolEntry {
                        id: exchange.call.provider_call_id,
                        name: exchange.call.tool_name,
                        arguments: exchange.call.arguments,
                        status: tool_status(&exchange.call.status).to_string(),
                        output: exchange
                            .result
                            .as_ref()
                            .map(|result| result.result_preview.clone())
                            .unwrap_or_default(),
                        ok: exchange.result.as_ref().map(|result| result.ok),
                        error: exchange
                            .result
                            .as_ref()
                            .and_then(|result| result.error.clone()),
                        result_ref: exchange
                            .result
                            .as_ref()
                            .and_then(|result| result.result_ref.clone()),
                        original_chars: exchange
                            .result
                            .as_ref()
                            .map(|result| result.original_chars),
                        created_at: exchange.call.created_at,
                        completed_at: exchange.result.map(|result| result.completed_at),
                    })
                    .collect();
                Ok(SessionTimelineTurn {
                    turn_id: turn.turn_id,
                    seq: turn.seq,
                    status: turn_status(turn.status).to_string(),
                    user: TimelineMessage {
                        timestamp: turn.user_timestamp,
                        content: turn.user_content,
                        reasoning: None,
                    },
                    assistant: TimelineMessage {
                        timestamp: turn.assistant_timestamp.unwrap_or_default(),
                        content: turn.assistant_content,
                        reasoning: turn.assistant_reasoning,
                    },
                    tools,
                })
            })
            .collect()
    }
}

/// 将工具状态转换为 Web 稳定文本。
///
/// 参数:
/// - `status`: 工具调用状态
///
/// 返回:
/// - 状态文本
fn tool_status(status: &ToolCallStatus) -> &'static str {
    match status {
        ToolCallStatus::Pending => "running",
        ToolCallStatus::Completed => "completed",
        ToolCallStatus::Error | ToolCallStatus::Interrupted => "failed",
    }
}

/// 将轮次状态转换为 Web 稳定文本。
///
/// 参数:
/// - `status`: 对话轮次状态
///
/// 返回:
/// - 状态文本
fn turn_status(status: TurnStatus) -> &'static str {
    match status {
        TurnStatus::Running => "running",
        TurnStatus::Completed => "completed",
        TurnStatus::Interrupted => "interrupted",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::MiyuPaths;
    use std::path::PathBuf;

    /// 创建时间线测试所需路径。
    ///
    /// 参数:
    /// - `root`: 临时根目录
    ///
    /// 返回:
    /// - Miyu 路径集合
    fn test_paths(root: PathBuf) -> MiyuPaths {
        MiyuPaths {
            config_dir: root.join("config"),
            config_file: root.join("config/config.jsonc"),
            secrets_file: root.join("config/secrets.jsonc"),
            skills_dir: root.join("config/skills"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            state_dir: root.join("state"),
            pictures_dir: root.join("pictures"),
            fish_hook_file: root.join("fish/miyu.fish"),
            bash_hook_file: root.join("shell/bash-hook.sh"),
            zsh_hook_file: root.join("shell/zsh-hook.zsh"),
            powershell_hook_file: root.join("shell/powershell-hook.ps1"),
        }
    }

    #[test]
    fn groups_tool_history_with_its_turn() {
        let temp = tempfile::tempdir().unwrap();
        let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
        store.start_turn("turn_1", "inspect").unwrap();
        store
            .record_tool_call_started("turn_1", 0, "call_1", "run_command", "{}")
            .unwrap();
        store
            .record_tool_result_completed("turn_1", "call_1", true, "ok", None, None, 2)
            .unwrap();
        store.complete_turn("turn_1", "done", None).unwrap();

        let timeline = store.session_timeline(10).unwrap();

        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].tools.len(), 1);
        assert_eq!(timeline[0].tools[0].name, "run_command");
        assert_eq!(timeline[0].tools[0].output, "ok");
    }
}
