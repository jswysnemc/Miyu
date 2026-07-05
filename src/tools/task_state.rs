use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tokio::sync::oneshot;

static TASKS: OnceLock<Mutex<HashMap<String, TaskRecord>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TaskSnapshot {
    pub(crate) id: String,
    pub(crate) description: String,
    pub(crate) subagent_type: String,
    pub(crate) status: String,
    pub(crate) max_steps: usize,
    pub(crate) started_at: u64,
    pub(crate) updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stats: Option<Value>,
}

struct TaskRecord {
    snapshot: TaskSnapshot,
    cancel: Option<oneshot::Sender<()>>,
}

/// 创建后台子代理任务记录。
///
/// 参数:
/// - `description`: 任务描述
/// - `subagent_type`: 子代理类型
/// - `max_steps`: 最大工具调用次数
///
/// 返回:
/// - 任务快照和取消接收器
pub(crate) fn create_task(
    description: String,
    subagent_type: String,
    max_steps: usize,
) -> (TaskSnapshot, oneshot::Receiver<()>) {
    let now = unix_seconds();
    let id = format!("task_{now}_{}", rand::random::<u16>());
    let (cancel_tx, cancel_rx) = oneshot::channel();
    let snapshot = TaskSnapshot {
        id: id.clone(),
        description,
        subagent_type,
        status: "running".to_string(),
        max_steps,
        started_at: now,
        updated_at: now,
        result: None,
        error: None,
        stats: None,
    };
    tasks().lock().expect("task state lock").insert(
        id,
        TaskRecord {
            snapshot: snapshot.clone(),
            cancel: Some(cancel_tx),
        },
    );
    (snapshot, cancel_rx)
}

/// 完成后台子代理任务记录。
///
/// 参数:
/// - `id`: 任务 ID
/// - `status`: 完成状态
/// - `result`: 子代理结果
/// - `error`: 错误信息
/// - `stats`: 统计信息
///
/// 返回:
/// - 无
pub(crate) fn finish_task(
    id: &str,
    status: &str,
    result: Option<String>,
    error: Option<String>,
    stats: Option<Value>,
) {
    let mut tasks = tasks().lock().expect("task state lock");
    let Some(record) = tasks.get_mut(id) else {
        return;
    };
    record.snapshot.status = status.to_string();
    record.snapshot.updated_at = unix_seconds();
    record.snapshot.result = result;
    record.snapshot.error = error;
    record.snapshot.stats = stats;
    record.cancel = None;
}

/// 读取后台子代理任务快照。
///
/// 参数:
/// - `id`: 任务 ID
///
/// 返回:
/// - 任务快照
pub(crate) fn task_snapshot(id: &str) -> Result<TaskSnapshot> {
    tasks()
        .lock()
        .expect("task state lock")
        .get(id)
        .map(|record| record.snapshot.clone())
        .ok_or_else(|| anyhow::anyhow!("subagent task not found: {id}"))
}

/// 列出后台子代理任务快照。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 任务快照列表
pub(crate) fn list_tasks() -> Vec<TaskSnapshot> {
    let mut tasks = tasks()
        .lock()
        .expect("task state lock")
        .values()
        .map(|record| record.snapshot.clone())
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| right.started_at.cmp(&left.started_at));
    tasks
}

/// 取消后台子代理任务。
///
/// 参数:
/// - `id`: 任务 ID
///
/// 返回:
/// - 取消后的任务快照
pub(crate) fn cancel_task(id: &str) -> Result<TaskSnapshot> {
    let mut tasks = tasks().lock().expect("task state lock");
    let record = tasks
        .get_mut(id)
        .ok_or_else(|| anyhow::anyhow!("subagent task not found: {id}"))?;
    if record.snapshot.status != "running" {
        return Ok(record.snapshot.clone());
    }
    if let Some(cancel) = record.cancel.take() {
        let _ = cancel.send(());
    } else {
        bail!("subagent task is not cancellable: {id}");
    }
    record.snapshot.status = "cancelled".to_string();
    record.snapshot.updated_at = unix_seconds();
    record.snapshot.error = Some("cancel requested".to_string());
    Ok(record.snapshot.clone())
}

/// 获取当前 Unix 秒数。
///
/// 参数:
/// - 无
///
/// 返回:
/// - Unix 秒数
fn unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

/// 获取后台子代理任务表。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 全局任务表
fn tasks() -> &'static Mutex<HashMap<String, TaskRecord>> {
    TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_reads_task_snapshot() {
        let (task, _cancel) = create_task("demo".to_string(), "explore".to_string(), 3);
        let loaded = task_snapshot(&task.id).unwrap();

        assert_eq!(loaded.description, "demo");
        assert_eq!(loaded.status, "running");
        assert_eq!(loaded.max_steps, 3);
    }

    #[test]
    fn cancel_marks_running_task_cancelled() {
        let (task, _cancel) = create_task("cancel".to_string(), "general".to_string(), 5);
        let cancelled = cancel_task(&task.id).unwrap();

        assert_eq!(cancelled.status, "cancelled");
    }
}
