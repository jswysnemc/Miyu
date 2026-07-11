use anyhow::{bail, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tokio::sync::oneshot;

static SUBAGENTS: OnceLock<Mutex<HashMap<String, SubagentRecord>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SubagentSnapshot {
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

struct SubagentRecord {
    snapshot: SubagentSnapshot,
    cancel: Option<oneshot::Sender<()>>,
}

/// 创建后台子智能体记录。
///
/// 参数:
/// - `description`: 任务描述
/// - `subagent_type`: 子代理类型
/// - `max_steps`: 最大工具调用次数
///
/// 返回:
/// - 子智能体快照和取消接收器
pub(crate) fn create_subagent(
    description: String,
    subagent_type: String,
    max_steps: usize,
) -> (SubagentSnapshot, oneshot::Receiver<()>) {
    let now = unix_seconds();
    let id = format!("subagent_{now}_{}", rand::random::<u16>());
    let (cancel_tx, cancel_rx) = oneshot::channel();
    let snapshot = SubagentSnapshot {
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
    subagents().lock().expect("subagent state lock").insert(
        id,
        SubagentRecord {
            snapshot: snapshot.clone(),
            cancel: Some(cancel_tx),
        },
    );
    (snapshot, cancel_rx)
}

/// 完成后台子智能体记录。
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
pub(crate) fn finish_subagent(
    id: &str,
    status: &str,
    result: Option<String>,
    error: Option<String>,
    stats: Option<Value>,
) {
    let mut subagents = subagents().lock().expect("subagent state lock");
    let Some(record) = subagents.get_mut(id) else {
        return;
    };
    record.snapshot.status = status.to_string();
    record.snapshot.updated_at = unix_seconds();
    record.snapshot.result = result;
    record.snapshot.error = error;
    record.snapshot.stats = stats;
    record.cancel = None;
}

/// 读取后台子智能体快照。
///
/// 参数:
/// - `id`: 任务 ID
///
/// 返回:
/// - 子智能体快照
pub(crate) fn subagent_snapshot(id: &str) -> Result<SubagentSnapshot> {
    subagents()
        .lock()
        .expect("subagent state lock")
        .get(id)
        .map(|record| record.snapshot.clone())
        .ok_or_else(|| anyhow::anyhow!("subagent not found: {id}"))
}

/// 列出后台子智能体快照。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 子智能体快照列表
pub(crate) fn list_subagents() -> Vec<SubagentSnapshot> {
    let mut subagents = subagents()
        .lock()
        .expect("subagent state lock")
        .values()
        .map(|record| record.snapshot.clone())
        .collect::<Vec<_>>();
    subagents.sort_by(|left, right| right.started_at.cmp(&left.started_at));
    subagents
}

/// 取消后台子智能体。
///
/// 参数:
/// - `id`: 任务 ID
///
/// 返回:
/// - 取消后的子智能体快照
pub(crate) fn cancel_subagent(id: &str) -> Result<SubagentSnapshot> {
    let mut subagents = subagents().lock().expect("subagent state lock");
    let record = subagents
        .get_mut(id)
        .ok_or_else(|| anyhow::anyhow!("subagent not found: {id}"))?;
    if record.snapshot.status != "running" {
        return Ok(record.snapshot.clone());
    }
    if let Some(cancel) = record.cancel.take() {
        let _ = cancel.send(());
    } else {
        bail!("subagent is not cancellable: {id}");
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

/// 获取后台子智能体表。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 全局子智能体表
fn subagents() -> &'static Mutex<HashMap<String, SubagentRecord>> {
    SUBAGENTS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_reads_subagent_snapshot() {
        let (subagent, _cancel) = create_subagent("demo".to_string(), "explore".to_string(), 3);
        let loaded = subagent_snapshot(&subagent.id).unwrap();

        assert_eq!(loaded.description, "demo");
        assert_eq!(loaded.status, "running");
        assert_eq!(loaded.max_steps, 3);
    }

    #[test]
    fn cancel_marks_running_subagent_cancelled() {
        let (subagent, _cancel) = create_subagent("cancel".to_string(), "general".to_string(), 5);
        let cancelled = cancel_subagent(&subagent.id).unwrap();

        assert_eq!(cancelled.status, "cancelled");
    }
}
