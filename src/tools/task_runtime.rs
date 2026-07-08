use super::task_state::TaskSnapshot;
use crate::paths::MiyuPaths;
use crate::runtime_recovery::{
    NewRuntimeProcessEventInput, NewRuntimeProcessRecord, OwnerKind, ProcessKind,
    RuntimeProcessStatus,
};
use crate::state::StateStore;
use anyhow::Result;

/// 记录子代理任务启动到 Runtime Recovery。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `task`: 子代理任务快照
///
/// 返回:
/// - 记录是否成功
pub(crate) fn record_subagent_task_started(paths: &MiyuPaths, task: &TaskSnapshot) -> Result<()> {
    let state = StateStore::new(paths)?;
    state.record_runtime_process(runtime_process(
        state.session_id(),
        task,
        RuntimeProcessStatus::Running,
    ))?;
    state.append_runtime_process_event(NewRuntimeProcessEventInput {
        process_id: runtime_process_id(&task.id),
        stream: "lifecycle".to_string(),
        event_kind: "started".to_string(),
        payload_ref: None,
        payload_preview: format!("subagent task started: {}", task.description),
    })?;
    Ok(())
}

/// 记录子代理任务结束到 Runtime Recovery。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `task`: 子代理任务快照
///
/// 返回:
/// - 记录是否成功
pub(crate) fn record_subagent_task_finished(paths: &MiyuPaths, task: &TaskSnapshot) -> Result<()> {
    let state = StateStore::new(paths)?;
    let status = runtime_status_from_task(&task.status);
    let seq = state.append_runtime_process_event(NewRuntimeProcessEventInput {
        process_id: runtime_process_id(&task.id),
        stream: "lifecycle".to_string(),
        event_kind: task.status.clone(),
        payload_ref: None,
        payload_preview: lifecycle_preview(task),
    })?;
    let mut process = runtime_process(state.session_id(), task, status);
    process.last_seq = seq;
    state.record_runtime_process(process)?;
    Ok(())
}

/// 创建子代理任务运行时进程记录。
///
/// 参数:
/// - `task`: 子代理任务快照
/// - `status`: 运行时进程状态
///
/// 返回:
/// - 运行时进程记录
fn runtime_process(
    session_id: &str,
    task: &TaskSnapshot,
    status: RuntimeProcessStatus,
) -> NewRuntimeProcessRecord {
    NewRuntimeProcessRecord {
        id: runtime_process_id(&task.id),
        session_id: session_id.to_string(),
        owner_kind: OwnerKind::Subagent,
        owner_id: task.id.clone(),
        process_kind: ProcessKind::SubagentTask,
        command: task.description.clone(),
        cwd: std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| ".".to_string()),
        pid: Some(i64::from(std::process::id())),
        pgid: None,
        status,
        last_seq: 0,
    }
}

/// 生成子代理任务运行时进程标识。
///
/// 参数:
/// - `task_id`: 子代理任务 ID
///
/// 返回:
/// - 运行时进程标识
fn runtime_process_id(task_id: &str) -> String {
    format!("subagent_task_{task_id}")
}

/// 将子代理任务状态映射为运行时进程状态。
///
/// 参数:
/// - `status`: 子代理任务状态
///
/// 返回:
/// - 运行时进程状态
fn runtime_status_from_task(status: &str) -> RuntimeProcessStatus {
    match status {
        "completed" => RuntimeProcessStatus::Exited,
        "cancelled" => RuntimeProcessStatus::Stopped,
        "failed" => RuntimeProcessStatus::Failed,
        _ => RuntimeProcessStatus::Running,
    }
}

/// 生成生命周期事件预览。
///
/// 参数:
/// - `task`: 子代理任务快照
///
/// 返回:
/// - 生命周期事件预览
fn lifecycle_preview(task: &TaskSnapshot) -> String {
    task.error.clone().unwrap_or_else(|| {
        format!(
            "subagent task {}: {}",
            task.status,
            task.result.as_deref().unwrap_or(&task.description)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::MiyuPaths;
    use crate::tools::task_state::TaskSnapshot;
    use std::path::PathBuf;

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

    fn task(status: &str) -> TaskSnapshot {
        TaskSnapshot {
            id: "task_1".to_string(),
            description: "inspect runtime recovery".to_string(),
            subagent_type: "explore".to_string(),
            status: status.to_string(),
            max_steps: 3,
            started_at: 1,
            updated_at: 1,
            result: None,
            error: None,
            stats: None,
        }
    }

    #[test]
    fn records_subagent_task_started_runtime_process() {
        let temp = tempfile::tempdir().unwrap();
        let paths = test_paths(temp.path().to_path_buf());

        record_subagent_task_started(&paths, &task("running")).unwrap();

        let db_path = crate::state::active_state_dir(&paths)
            .unwrap()
            .join("conversation.db");
        let conn = rusqlite::Connection::open(db_path).unwrap();
        let (owner_kind, owner_id, process_kind, status, last_seq): (
            String,
            String,
            String,
            String,
            i64,
        ) = conn
            .query_row(
                "SELECT owner_kind, owner_id, process_kind, status, last_seq
                 FROM runtime_processes
                 WHERE id = ?1",
                ["subagent_task_task_1"],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(owner_kind, "subagent");
        assert_eq!(owner_id, "task_1");
        assert_eq!(process_kind, "subagent_task");
        assert_eq!(status, "running");
        assert_eq!(last_seq, 1);
    }
}
