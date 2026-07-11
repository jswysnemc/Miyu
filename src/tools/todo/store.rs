use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// TODO 项状态。
#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl TodoStatus {
    /// 解析工具参数中的状态。
    ///
    /// 参数:
    /// - `value`: 状态文本
    ///
    /// 返回:
    /// - 解析后的状态
    pub(crate) fn parse(value: &str) -> Result<Self> {
        match value.trim() {
            "pending" => Ok(Self::Pending),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            other => bail!("unsupported todo status: {other}"),
        }
    }

    /// 判断当前状态是否仍未完成。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 是否属于未完成状态
    pub(crate) fn is_unfinished(self) -> bool {
        matches!(self, Self::Pending | Self::InProgress)
    }
}

/// 会话级 TODO 项。
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct TodoItem {
    pub(crate) id: String,
    pub(crate) text: String,
    pub(crate) status: TodoStatus,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

/// 会话级 TODO 持久化存储。
#[derive(Debug, Clone)]
pub(crate) struct TodoStore {
    file: PathBuf,
}

impl TodoStore {
    /// 创建绑定指定状态文件的 TODO 存储。
    ///
    /// 参数:
    /// - `file`: 当前会话 TODO 文件
    ///
    /// 返回:
    /// - TODO 存储
    pub(crate) fn new(file: PathBuf) -> Self {
        Self { file }
    }

    /// 读取当前会话全部 TODO 项。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - TODO 项列表
    pub(crate) fn list(&self) -> Result<Vec<TodoItem>> {
        if !self.file.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&self.file)
            .with_context(|| format!("failed to read todo file {}", self.file.display()))?;
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(&content)
            .with_context(|| format!("failed to parse todo file {}", self.file.display()))
    }

    /// 新增一个 TODO 项。
    ///
    /// 参数:
    /// - `text`: TODO 内容
    ///
    /// 返回:
    /// - 新增后的 TODO 项
    pub(crate) fn add(&self, text: &str) -> Result<TodoItem> {
        let text = required_text(text)?;
        let mut items = self.list()?;
        let now = Utc::now().to_rfc3339();
        let item = TodoItem {
            id: format!(
                "todo_{}_{}",
                Utc::now().timestamp_millis(),
                rand::random::<u16>()
            ),
            text: text.to_string(),
            status: TodoStatus::Pending,
            created_at: now.clone(),
            updated_at: now,
        };
        items.push(item.clone());
        self.save(&items)?;
        Ok(item)
    }

    /// 更新指定 TODO 项。
    ///
    /// 参数:
    /// - `id`: TODO 标识
    /// - `text`: 可选的新内容
    /// - `status`: 可选的新状态
    ///
    /// 返回:
    /// - 更新后的 TODO 项
    pub(crate) fn update(
        &self,
        id: &str,
        text: Option<&str>,
        status: Option<TodoStatus>,
    ) -> Result<TodoItem> {
        if text.is_none() && status.is_none() {
            bail!("todo update requires text or status")
        }
        let mut items = self.list()?;
        let index = items
            .iter()
            .position(|item| item.id == id.trim())
            .with_context(|| format!("todo item not found: {}", id.trim()))?;
        if let Some(next_status) = status {
            validate_transition(&items, index, next_status)?;
        }
        let item = &mut items[index];
        if let Some(text) = text {
            item.text = required_text(text)?.to_string();
        }
        if let Some(status) = status {
            item.status = status;
        }
        item.updated_at = Utc::now().to_rfc3339();
        let updated = item.clone();
        self.save(&items)?;
        Ok(updated)
    }

    /// 删除指定 TODO 项。
    ///
    /// 参数:
    /// - `id`: TODO 标识
    ///
    /// 返回:
    /// - 被删除的 TODO 项
    pub(crate) fn remove(&self, id: &str) -> Result<TodoItem> {
        let mut items = self.list()?;
        let index = items
            .iter()
            .position(|item| item.id == id.trim())
            .with_context(|| format!("todo item not found: {}", id.trim()))?;
        let removed = items.remove(index);
        self.save(&items)?;
        Ok(removed)
    }

    /// 判断当前会话是否存在未完成项。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 是否存在未完成项
    pub(crate) fn has_unfinished(&self) -> Result<bool> {
        Ok(self.list()?.iter().any(|item| item.status.is_unfinished()))
    }

    /// 保存当前会话全部 TODO 项。
    ///
    /// 参数:
    /// - `items`: TODO 项列表
    ///
    /// 返回:
    /// - 保存是否成功
    fn save(&self, items: &[TodoItem]) -> Result<()> {
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(items)?;
        std::fs::write(&self.file, format!("{content}\n"))
            .with_context(|| format!("failed to write todo file {}", self.file.display()))
    }

    /// 返回当前存储文件路径。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - TODO 文件路径
    #[cfg(test)]
    pub(crate) fn file(&self) -> &std::path::Path {
        &self.file
    }
}

/// 校验 TODO 必须按照列表顺序逐项推进。
fn validate_transition(items: &[TodoItem], index: usize, next: TodoStatus) -> Result<()> {
    if matches!(next, TodoStatus::InProgress | TodoStatus::Completed)
        && items[..index]
            .iter()
            .any(|item| item.status.is_unfinished())
    {
        bail!("complete earlier todo items before advancing this item")
    }
    if next == TodoStatus::InProgress
        && items.iter().enumerate().any(|(other_index, item)| {
            other_index != index && item.status == TodoStatus::InProgress
        })
    {
        bail!("only one todo item can be in progress")
    }
    if next == TodoStatus::Completed && items[index].status != TodoStatus::InProgress {
        bail!("todo item must be in progress before completion")
    }
    Ok(())
}

/// 校验并返回非空 TODO 内容。
///
/// 参数:
/// - `text`: 待校验内容
///
/// 返回:
/// - 去除首尾空白后的内容
fn required_text(text: &str) -> Result<&str> {
    let text = text.trim();
    if text.is_empty() {
        bail!("todo text is required")
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 TODO 项能够持久化并更新。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn persists_and_updates_items() {
        let dir = tempfile::tempdir().unwrap();
        let store = TodoStore::new(dir.path().join("todos.json"));

        let item = store.add("实现持久化").unwrap();
        assert!(store.file().exists());
        assert!(store.has_unfinished().unwrap());

        store
            .update(&item.id, None, Some(TodoStatus::InProgress))
            .unwrap();
        let updated = store
            .update(&item.id, None, Some(TodoStatus::Completed))
            .unwrap();
        assert_eq!(updated.status, TodoStatus::Completed);
        assert!(!store.has_unfinished().unwrap());

        let reopened = TodoStore::new(store.file().to_path_buf());
        assert_eq!(reopened.list().unwrap(), vec![updated]);
    }

    /// 验证不同会话文件相互隔离。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn isolates_items_by_session_file() {
        let dir = tempfile::tempdir().unwrap();
        let first = TodoStore::new(dir.path().join("first/todos.json"));
        let second = TodoStore::new(dir.path().join("second/todos.json"));

        first.add("first session").unwrap();

        assert_eq!(first.list().unwrap().len(), 1);
        assert!(second.list().unwrap().is_empty());
    }

    #[test]
    fn enforces_sequential_single_in_progress_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let store = TodoStore::new(dir.path().join("todos.json"));
        let first = store.add("first").unwrap();
        let second = store.add("second").unwrap();

        assert!(store
            .update(&second.id, None, Some(TodoStatus::InProgress))
            .is_err());
        store
            .update(&first.id, None, Some(TodoStatus::InProgress))
            .unwrap();
        assert!(store
            .update(&second.id, None, Some(TodoStatus::InProgress))
            .is_err());
        store
            .update(&first.id, None, Some(TodoStatus::Completed))
            .unwrap();
        store
            .update(&second.id, None, Some(TodoStatus::InProgress))
            .unwrap();
    }
}
