use super::manager::{ActiveRunInfo, StartRunRequest};
use crate::paths::MiyuPaths;
use crate::web::workspaces::WorkspaceInfo;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const CHECKPOINT_FILE: &str = "web/run-checkpoints.json";

/// Web 运行检查点状态。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RunCheckpointStatus {
    Queued,
    Running,
    Completed,
    Interrupted,
    Failed,
}

/// 可在进程重启后恢复的运行检查点。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RunCheckpoint {
    pub(crate) info: ActiveRunInfo,
    pub(crate) workspace: WorkspaceInfo,
    pub(crate) request: StartRunRequest,
    pub(crate) status: RunCheckpointStatus,
    pub(crate) updated_at: String,
}

/// 保存运行请求、队列状态和终态。
#[derive(Clone)]
pub(crate) struct RunCheckpointStore {
    path: PathBuf,
    event_dir: PathBuf,
    records: Arc<Mutex<Vec<RunCheckpoint>>>,
}

impl RunCheckpointStore {
    /// 读取运行检查点存储。
    ///
    /// 参数:
    /// - `paths`: Miyu 路径集合
    ///
    /// 返回:
    /// - 运行检查点存储
    pub(crate) fn new(paths: &MiyuPaths) -> Result<Self> {
        let path = paths.state_dir.join(CHECKPOINT_FILE);
        let records = if path.is_file() {
            serde_json::from_slice(&std::fs::read(&path)?)?
        } else {
            Vec::new()
        };
        Ok(Self {
            path,
            event_dir: paths.state_dir.join("web/run-events"),
            records: Arc::new(Mutex::new(records)),
        })
    }

    /// 新增或替换运行检查点。
    pub(crate) fn upsert(&self, mut checkpoint: RunCheckpoint) -> Result<()> {
        checkpoint.updated_at = chrono::Utc::now().to_rfc3339();
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        records.retain(|record| record.info.run_id != checkpoint.info.run_id);
        records.push(checkpoint);
        self.save_locked(&records)
    }

    /// 更新指定运行状态。
    pub(crate) fn update_status(&self, run_id: &str, status: RunCheckpointStatus) -> Result<()> {
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(record) = records
            .iter_mut()
            .find(|record| record.info.run_id == run_id)
        {
            record.status = status;
            record.info.status = status;
            record.updated_at = chrono::Utc::now().to_rfc3339();
        }
        self.save_locked(&records)
    }

    /// 返回指定运行检查点。
    pub(crate) fn get(&self, run_id: &str) -> Option<RunCheckpoint> {
        self.records
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .find(|record| record.info.run_id == run_id)
            .cloned()
    }

    /// 返回等待恢复的排队运行。
    pub(crate) fn queued(&self) -> Vec<RunCheckpoint> {
        self.records
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .filter(|record| record.status == RunCheckpointStatus::Queued)
            .cloned()
            .collect()
    }

    /// 将进程退出时仍在运行的检查点恢复为中断状态。
    pub(crate) fn recover_running_as_interrupted(&self) -> Result<Vec<RunCheckpoint>> {
        let mut records = self
            .records
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut recovered = Vec::new();
        for record in records.iter_mut() {
            if record.status != RunCheckpointStatus::Running {
                continue;
            }
            record.status = RunCheckpointStatus::Interrupted;
            record.info.status = RunCheckpointStatus::Interrupted;
            record.updated_at = chrono::Utc::now().to_rfc3339();
            recovered.push(record.clone());
        }
        self.save_locked(&records)?;
        Ok(recovered)
    }

    /// 返回指定运行的事件日志文件。
    pub(crate) fn event_path(&self, run_id: &str) -> PathBuf {
        self.event_dir.join(format!("{run_id}.jsonl"))
    }

    /// 原子保存全部检查点。
    fn save_locked(&self, records: &[RunCheckpoint]) -> Result<()> {
        let parent = self
            .path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        std::fs::create_dir_all(parent)?;
        let temp = tempfile::NamedTempFile::new_in(parent)?;
        std::fs::write(temp.path(), serde_json::to_vec_pretty(records)?)?;
        temp.persist(&self.path)?;
        Ok(())
    }
}
