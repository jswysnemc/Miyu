mod compaction;
mod loaded_tools;
mod pending_turn;
mod sessions;
mod turns;
mod usage;

use crate::llm::Usage;
use crate::paths::MiyuPaths;
use anyhow::Result;
#[cfg(test)]
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::Arc;

pub use compaction::CompactionRequest;
pub use pending_turn::PendingTurnGuard;
pub use sessions::{
    create_session, delete_session, ensure_active_session as active_session, list_sessions,
    rename_session, switch_session,
};
#[cfg(test)]
pub use turns::TurnStatus;
pub use turns::{turns_to_entries, ConversationDb, StoredConversationEntry, Turn};

#[derive(Debug, Clone)]
pub struct StateStore {
    base_state_dir: PathBuf,
    session_id: String,
    state_dir: PathBuf,
    conv_db: Arc<ConversationDb>,
}

impl StateStore {
    /// 创建状态存储并迁移旧对话历史。
    ///
    /// 参数:
    /// - `paths`: Miyu 路径集合
    ///
    /// 返回:
    /// - 状态存储
    pub fn new(paths: &MiyuPaths) -> Result<Self> {
        let session = sessions::ensure_active_session(paths)?;
        let base_state_dir = sessions::session_scope_dir(paths)?;
        let state_dir = sessions::active_state_dir(paths)?;
        let conv_db = Arc::new(ConversationDb::open(&state_dir)?);
        let store = Self {
            base_state_dir,
            session_id: session.id,
            state_dir,
            conv_db,
        };
        store.migrate_from_jsonl()?;
        Ok(store)
    }

    /// 初始化状态文件。
    ///
    /// 返回:
    /// - 初始化是否成功
    pub fn init_files(&self) -> Result<()> {
        std::fs::create_dir_all(&self.state_dir)?;
        if !self.usage_file().exists() {
            std::fs::write(self.usage_file(), "{\n  \"requests\": 0,\n  \"prompt_tokens\": 0,\n  \"completion_tokens\": 0,\n  \"total_tokens\": 0\n}\n")?;
        }
        touch(self.log_file())?;
        if !self.profile_file().exists() {
            std::fs::write(self.profile_file(), "# Miyu Profile\n\n")?;
        }
        Ok(())
    }

    /// 系统提示变化时重置会话。
    ///
    /// 参数:
    /// - `system_prompt`: 当前系统提示
    ///
    /// 返回:
    /// - 重置检查是否成功
    pub fn reset_if_prompt_changed(&self, system_prompt: &str) -> Result<()> {
        self.init_files()?;
        let fingerprint = prompt_fingerprint(system_prompt);
        let file = self.prompt_fingerprint_file();
        let previous = std::fs::read_to_string(&file).unwrap_or_default();
        if previous.trim() != fingerprint {
            self.conv_db.reset()?;
            self.clear_loaded_tools()?;
            self.clear_compaction_summary()?;
            self.clear_last_usage()?;
            std::fs::write(file, format!("{fingerprint}\n"))?;
        }
        Ok(())
    }

    /// 开始新对话轮次。
    ///
    /// 参数:
    /// - `turn_id`: 当前轮唯一标识
    /// - `user_content`: 用户输入
    ///
    /// 返回:
    /// - 写入是否成功
    pub fn start_turn(&self, turn_id: &str, user_content: &str) -> Result<()> {
        self.conv_db.start_turn(turn_id, user_content)?;
        sessions::touch_session_with_message(&self.base_state_dir, &self.session_id, user_content)
    }

    /// 完成对话轮次。
    ///
    /// 参数:
    /// - `turn_id`: 当前轮唯一标识
    /// - `content`: 助手回复
    /// - `reasoning`: 可选推理内容
    ///
    /// 返回:
    /// - 写入是否成功
    pub fn complete_turn(
        &self,
        turn_id: &str,
        content: &str,
        reasoning: Option<&str>,
    ) -> Result<()> {
        self.conv_db.complete_turn(turn_id, content, reasoning)
    }

    /// 中断对话轮次。
    ///
    /// 参数:
    /// - `turn_id`: 当前轮唯一标识
    ///
    /// 返回:
    /// - 写入是否成功
    pub fn interrupt_turn(&self, turn_id: &str) -> Result<()> {
        self.conv_db.interrupt_turn(turn_id)
    }

    /// 附加工具报告上下文。
    ///
    /// 参数:
    /// - `turn_id`: 当前轮唯一标识
    /// - `tool_name`: 工具名称
    /// - `report`: 工具报告
    ///
    /// 返回:
    /// - 写入是否成功
    pub fn append_tool_report_context(
        &self,
        turn_id: &str,
        tool_name: &str,
        report: &str,
    ) -> Result<()> {
        self.conv_db.append_tool_report(
            turn_id,
            &format!(
                "<previous_tool_report name=\"{tool_name}\">\n{}\n</previous_tool_report>",
                report.trim()
            ),
        )
    }

    /// 恢复运行中的旧轮次为中断状态。
    ///
    /// 返回:
    /// - 被恢复轮次数量
    pub fn recover_stale_turns(&self) -> Result<usize> {
        self.conv_db.recover_stale_running_turns()
    }

    /// 兼容旧 JSONL 孤立用户消息检查。
    ///
    /// 返回:
    /// - 是否标记了中断轮次
    #[cfg(test)]
    pub fn mark_interrupted_turn_if_needed(&self) -> Result<bool> {
        let recovered = self.recover_stale_turns()?;
        Ok(recovered > 0)
    }

    /// 读取最近历史入口。
    ///
    /// 参数:
    /// - `limit`: 最大入口数量
    ///
    /// 返回:
    /// - 历史入口
    pub fn history(&self, limit: usize) -> Result<Vec<StoredConversationEntry>> {
        let mut entries = self.load_conversation()?;
        let start = entries.len().saturating_sub(limit);
        Ok(entries.split_off(start))
    }

    /// 读取完整对话历史入口。
    ///
    /// 返回:
    /// - 旧消息入口视图
    pub fn load_conversation(&self) -> Result<Vec<StoredConversationEntry>> {
        Ok(turns_to_entries(self.conv_db.load_turns()?))
    }

    /// 读取完整对话轮次。
    ///
    /// 返回:
    /// - 轮次列表
    #[cfg(test)]
    pub fn load_turns(&self) -> Result<Vec<Turn>> {
        self.conv_db.load_turns()
    }

    /// 读取排除指定轮次后的上下文轮次。
    ///
    /// 参数:
    /// - `exclude_turn_id`: 排除轮次标识
    ///
    /// 返回:
    /// - 轮次列表
    pub fn load_turns_excluding(&self, exclude_turn_id: &str) -> Result<Vec<Turn>> {
        self.conv_db.load_turns_excluding(exclude_turn_id)
    }

    /// 清空对话历史。
    ///
    /// 返回:
    /// - 清空是否成功
    pub fn reset_conversation(&self) -> Result<()> {
        self.conv_db.reset()?;
        self.clear_loaded_tools()?;
        self.clear_compaction_summary()?;
        self.clear_last_usage()
    }

    /// 读取当前会话已经载入的工具集合。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 已载入工具名称列表
    pub fn load_loaded_tools(&self) -> Result<Vec<String>> {
        loaded_tools::load(&self.loaded_tools_file())
    }

    /// 保存当前会话已经载入的工具集合。
    ///
    /// 参数:
    /// - `names`: 已载入工具名称列表
    ///
    /// 返回:
    /// - 保存是否成功
    pub fn save_loaded_tools(&self, names: &[String]) -> Result<()> {
        loaded_tools::save(&self.loaded_tools_file(), names)
    }

    /// 清空当前会话已经载入的工具集合。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 清空是否成功
    pub fn clear_loaded_tools(&self) -> Result<()> {
        loaded_tools::clear(&self.loaded_tools_file())
    }

    /// 撤销最后一轮对话。
    ///
    /// 返回:
    /// - 删除轮次数量和被撤销的用户输入
    pub fn undo_last_turn(&self) -> Result<(usize, Option<String>)> {
        self.conv_db.undo_last_turn()
    }

    /// 累加用量统计。
    ///
    /// 参数:
    /// - `usage`: 模型用量
    ///
    /// 返回:
    /// - 写入是否成功
    pub fn add_usage(&self, usage: &Usage) -> Result<()> {
        self.init_files()?;
        usage::add_usage(&self.usage_file(), usage)
    }

    /// 读取最近一次 provider usage。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 最近一次 provider usage
    fn last_usage(&self) -> Result<Option<Usage>> {
        usage::last_usage(&self.usage_file())
    }

    /// 清空最近一次 provider usage。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 清空是否成功
    fn clear_last_usage(&self) -> Result<()> {
        usage::clear_last_usage(&self.usage_file())
    }

    /// 是否存在运行中轮次。
    ///
    /// 返回:
    /// - 是否存在运行中轮次
    #[allow(dead_code)]
    pub fn has_running_turns(&self) -> Result<bool> {
        self.conv_db.has_running_turns()
    }

    /// 从旧 JSONL 文件迁移历史。
    ///
    /// 返回:
    /// - 迁移轮次数量
    pub fn migrate_from_jsonl(&self) -> Result<usize> {
        self.conv_db.migrate_from_jsonl(&self.conversation_file())
    }

    /// 兼容旧测试和辅助代码追加消息。
    ///
    /// 参数:
    /// - `role`: 消息角色
    /// - `content`: 消息内容
    ///
    /// 返回:
    /// - 写入是否成功
    #[cfg(test)]
    pub fn append_message(&self, role: &str, content: &str) -> Result<()> {
        match role {
            "user" => self.start_turn(&compat_turn_id(), content),
            "assistant" => self.append_assistant_message(content, None),
            _ => Ok(()),
        }
    }

    /// 兼容旧测试和辅助代码追加助手消息。
    ///
    /// 参数:
    /// - `content`: 助手回复
    /// - `reasoning`: 可选推理内容
    ///
    /// 返回:
    /// - 写入是否成功
    #[cfg(test)]
    pub fn append_assistant_message(&self, content: &str, reasoning: Option<&str>) -> Result<()> {
        if let Some(turn) = self
            .conv_db
            .load_turns()?
            .into_iter()
            .rev()
            .find(|turn| turn.status == TurnStatus::Running)
        {
            self.complete_turn(&turn.turn_id, content, reasoning)?;
        }
        Ok(())
    }

    fn conversation_file(&self) -> PathBuf {
        self.state_dir.join("conversation.jsonl")
    }

    fn usage_file(&self) -> PathBuf {
        self.state_dir.join("usage.json")
    }

    fn loaded_tools_file(&self) -> PathBuf {
        self.state_dir.join("loaded-tools.json")
    }

    fn log_file(&self) -> PathBuf {
        self.state_dir.join("miyu.log")
    }

    fn profile_file(&self) -> PathBuf {
        self.state_dir.join("profile.md")
    }

    fn compaction_summary_file(&self) -> PathBuf {
        self.state_dir.join("compaction-summary.json")
    }

    fn prompt_fingerprint_file(&self) -> PathBuf {
        self.state_dir.join("prompt.sha256")
    }
}

/// 计算系统提示指纹。
///
/// 参数:
/// - `system_prompt`: 系统提示
///
/// 返回:
/// - 十六进制指纹
fn prompt_fingerprint(system_prompt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(system_prompt.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 创建兼容写入使用的轮次标识。
///
/// 返回:
/// - 轮次标识
#[cfg(test)]
fn compat_turn_id() -> String {
    format!(
        "compat_{}_{}",
        Utc::now().timestamp_millis(),
        rand::random::<u16>()
    )
}

/// 确保文件存在。
///
/// 参数:
/// - `path`: 文件路径
///
/// 返回:
/// - 创建是否成功
fn touch(path: PathBuf) -> Result<()> {
    OpenOptions::new().create(true).append(true).open(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::turns::pending_placeholder;
    use super::*;

    fn test_paths(state_dir: PathBuf) -> MiyuPaths {
        MiyuPaths {
            config_dir: PathBuf::new(),
            config_file: PathBuf::new(),
            secrets_file: PathBuf::new(),
            skills_dir: PathBuf::new(),
            data_dir: PathBuf::new(),
            cache_dir: PathBuf::new(),
            state_dir,
            pictures_dir: PathBuf::new(),
            fish_hook_file: PathBuf::new(),
            bash_hook_file: PathBuf::new(),
            zsh_hook_file: PathBuf::new(),
            powershell_hook_file: PathBuf::new(),
        }
    }

    #[test]
    fn turn_lifecycle() {
        let temp = tempfile::tempdir().unwrap();
        let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
        store.start_turn("turn_1", "hello").unwrap();
        let turns = store.load_turns().unwrap();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].status, TurnStatus::Running);
        assert_eq!(turns[0].assistant_content, pending_placeholder());

        store.complete_turn("turn_1", "hi there", None).unwrap();
        let turns = store.load_turns().unwrap();
        assert_eq!(turns[0].status, TurnStatus::Completed);
        assert_eq!(turns[0].assistant_content, "hi there");
    }

    #[test]
    fn marks_running_turns_as_interrupted() {
        let temp = tempfile::tempdir().unwrap();
        let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
        store.start_turn("turn_1", "old task").unwrap();
        assert!(store.mark_interrupted_turn_if_needed().unwrap());
        let turns = store.load_turns().unwrap();
        assert_eq!(turns[0].status, TurnStatus::Interrupted);
        assert!(turns[0].assistant_content.contains("被中断"));
        assert!(!store.mark_interrupted_turn_if_needed().unwrap());
    }

    #[test]
    fn undo_removes_last_turn() {
        let temp = tempfile::tempdir().unwrap();
        let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
        store.start_turn("turn_1", "hello").unwrap();
        store.complete_turn("turn_1", "hi", None).unwrap();
        store.start_turn("turn_2", "bye").unwrap();
        store.complete_turn("turn_2", "goodbye", None).unwrap();

        let (removed, prompt) = store.undo_last_turn().unwrap();
        assert_eq!(removed, 1);
        assert_eq!(prompt.as_deref(), Some("bye"));
        assert_eq!(store.load_turns().unwrap().len(), 1);
    }

    #[test]
    fn reset_conversation_clears_loaded_tools() {
        let temp = tempfile::tempdir().unwrap();
        let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();

        store
            .save_loaded_tools(&["web_search".to_string(), "web_fetch".to_string()])
            .unwrap();
        assert_eq!(
            store.load_loaded_tools().unwrap(),
            vec!["web_fetch".to_string(), "web_search".to_string()]
        );

        store.reset_conversation().unwrap();

        assert!(store.load_loaded_tools().unwrap().is_empty());
    }

    #[test]
    fn compaction_summary_is_applied_and_injected() {
        let temp = tempfile::tempdir().unwrap();
        let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
        for index in 1..=4 {
            let turn_id = format!("turn_{index}");
            store.start_turn(&turn_id, &"u".repeat(200)).unwrap();
            store
                .complete_turn(&turn_id, &"a".repeat(200), None)
                .unwrap();
        }
        store
            .add_usage(&Usage {
                prompt_tokens: 900,
                completion_tokens: 20,
                total_tokens: 920,
            })
            .unwrap();

        let request = store
            .select_compaction(1000, 0.5)
            .unwrap()
            .expect("compaction request");
        store
            .apply_compaction(&request, "## Goal\n- keep context")
            .unwrap();

        let turns = store.load_turns().unwrap();
        let context = store.compaction_summary_context().unwrap().unwrap();

        assert_eq!(turns.len(), 2);
        assert!(context.contains("<conversation-summary>"));
        assert!(context.contains("keep context"));
    }

    #[test]
    fn reset_conversation_clears_compaction_summary() {
        let temp = tempfile::tempdir().unwrap();
        let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
        for index in 1..=4 {
            let turn_id = format!("turn_{index}");
            store.start_turn(&turn_id, &"u".repeat(200)).unwrap();
            store
                .complete_turn(&turn_id, &"a".repeat(200), None)
                .unwrap();
        }
        store
            .add_usage(&Usage {
                prompt_tokens: 900,
                completion_tokens: 20,
                total_tokens: 920,
            })
            .unwrap();
        let request = store
            .select_compaction(1000, 0.5)
            .unwrap()
            .expect("compaction request");
        store.apply_compaction(&request, "summary").unwrap();

        store.reset_conversation().unwrap();

        assert!(store.compaction_summary_context().unwrap().is_none());
    }

    #[test]
    fn compaction_skips_without_provider_usage() {
        let temp = tempfile::tempdir().unwrap();
        let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
        for index in 1..=4 {
            let turn_id = format!("turn_{index}");
            store.start_turn(&turn_id, &"u".repeat(200)).unwrap();
            store
                .complete_turn(&turn_id, &"a".repeat(200), None)
                .unwrap();
        }

        assert!(store.select_compaction(1000, 0.5).unwrap().is_none());
    }

    #[test]
    fn compaction_skips_when_provider_usage_is_under_threshold() {
        let temp = tempfile::tempdir().unwrap();
        let store = StateStore::new(&test_paths(temp.path().to_path_buf())).unwrap();
        for index in 1..=4 {
            let turn_id = format!("turn_{index}");
            store.start_turn(&turn_id, &"u".repeat(200)).unwrap();
            store
                .complete_turn(&turn_id, &"a".repeat(200), None)
                .unwrap();
        }
        store
            .add_usage(&Usage {
                prompt_tokens: 200,
                completion_tokens: 20,
                total_tokens: 220,
            })
            .unwrap();

        assert!(store.select_compaction(1000, 0.5).unwrap().is_none());
    }

    #[test]
    fn sessions_have_isolated_conversations() {
        let temp = tempfile::tempdir().unwrap();
        let paths = test_paths(temp.path().to_path_buf());
        let default_store = StateStore::new(&paths).unwrap();
        default_store.start_turn("turn_default", "default").unwrap();
        default_store
            .complete_turn("turn_default", "default reply", None)
            .unwrap();

        let session = create_session(&paths, Some("work")).unwrap();
        let work_store = StateStore::new(&paths).unwrap();
        assert!(work_store.load_conversation().unwrap().is_empty());
        work_store.start_turn("turn_work", "work").unwrap();
        work_store
            .complete_turn("turn_work", "work reply", None)
            .unwrap();

        switch_session(&paths, "default").unwrap();
        let default_store = StateStore::new(&paths).unwrap();
        let default_history = default_store.load_conversation().unwrap();

        switch_session(&paths, &session.id).unwrap();
        let work_store = StateStore::new(&paths).unwrap();
        let work_history = work_store.load_conversation().unwrap();

        assert!(default_history
            .iter()
            .any(|entry| entry.content == "default"));
        assert!(!default_history.iter().any(|entry| entry.content == "work"));
        assert!(work_history.iter().any(|entry| entry.content == "work"));
        assert!(!work_history.iter().any(|entry| entry.content == "default"));
    }
}
