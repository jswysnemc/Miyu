use super::background_timeout::{is_unlimited, timeout_seconds_from_args};
use super::process::{process_exists, spawn_background_shell, terminate_process};
use super::store::{unix_seconds, BackgroundCommandStore, BackgroundCommandTask};
use crate::config::AppConfig;
use crate::i18n::text as t;
use crate::paths::MiyuPaths;
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// 启动后台命令。
///
/// 参数:
/// - `args`: 工具参数
/// - `config`: 应用配置
/// - `paths`: Miyu 路径
/// - `allowed`: 是否允许命令执行
///
/// 返回:
/// - JSON 格式任务信息
pub(super) fn start_background_task(
    args: Value,
    config: &AppConfig,
    paths: &MiyuPaths,
    allowed: bool,
) -> Result<String> {
    if !allowed {
        bail!("{}", t("command execution is disabled; set skills.allow_command_execution=true in config.jsonc to enable background commands", "命令执行已禁用；请在 config.jsonc 中设置 skills.allow_command_execution=true 以启用后台命令"));
    }
    if !config.tools.background_commands_enabled {
        bail!(
            "{}",
            t("background commands are disabled", "后台命令已禁用")
        );
    }
    let command = required(&args, "command")?;
    let cwd = args
        .get("cwd")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(expand_path)
        .unwrap_or(std::env::current_dir()?);
    if !cwd.is_dir() {
        bail!(
            "{}: {}",
            t(
                "background command cwd is not a directory",
                "后台命令工作目录不是目录"
            ),
            cwd.display()
        );
    }
    let label = args
        .get("label")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("background command")
        .to_string();
    let timeout_seconds =
        timeout_seconds_from_args(&args, config.tools.background_command_timeout_seconds);
    let store = BackgroundCommandStore::new(paths.state_dir.clone());
    store.init()?;
    let now = unix_seconds();
    let id_prefix = sanitize_id(&label);
    let stdout_log = store.logs_dir().join(format!("{now}-{id_prefix}.out.log"));
    let stderr_log = store.logs_dir().join(format!("{now}-{id_prefix}.err.log"));
    let stdout = std::fs::File::create(&stdout_log)?;
    let stderr = std::fs::File::create(&stderr_log)?;
    let process = spawn_background_shell(&command, &cwd, stdout, stderr)?;
    let task_id = format!("{now}-{}", process.pid);
    let task = BackgroundCommandTask {
        id: task_id.clone(),
        label,
        command,
        cwd: cwd.display().to_string(),
        pid: process.pid,
        pgid: process.pgid,
        status: "running".to_string(),
        stdout_log: stdout_log.display().to_string(),
        stderr_log: stderr_log.display().to_string(),
        started_at: now,
        updated_at: now,
        timeout_seconds,
    };
    store.upsert(task.clone())?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "task": task,
        "note": "Use background_command with action=list, action=output, action=stop, or action=cleanup to manage this task."
    }))?)
}

/// 列出后台命令。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `config`: 应用配置
///
/// 返回:
/// - JSON 格式任务列表
pub(super) async fn list_background_tasks(paths: &MiyuPaths, config: &AppConfig) -> Result<String> {
    let store = BackgroundCommandStore::new(paths.state_dir.clone());
    let mut tasks = store.load()?;
    refresh_task_statuses(&mut tasks, config).await;
    store.save(&tasks)?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "tasks": tasks,
    }))?)
}

/// 读取后台命令输出。
///
/// 参数:
/// - `args`: 工具参数
/// - `config`: 应用配置
/// - `paths`: Miyu 路径
///
/// 返回:
/// - JSON 格式输出
pub(super) async fn read_background_task_output(
    args: Value,
    config: &AppConfig,
    paths: &MiyuPaths,
) -> Result<String> {
    let task_id = required(&args, "task_id")?;
    let stream = args
        .get("stream")
        .and_then(Value::as_str)
        .unwrap_or("all")
        .trim();
    let tail_lines = args
        .get("tail_lines")
        .and_then(Value::as_u64)
        .unwrap_or(200)
        .clamp(1, 2000) as usize;
    let store = BackgroundCommandStore::new(paths.state_dir.clone());
    let mut tasks = store.load()?;
    refresh_task_statuses(&mut tasks, config).await;
    store.save(&tasks)?;
    let task = find_task(&tasks, &task_id)?;
    let max_bytes = config.tools.background_command_log_max_bytes;
    let stdout = if matches!(stream, "stdout" | "all") {
        Some(read_log_tail(&task.stdout_log, tail_lines, max_bytes)?)
    } else {
        None
    };
    let stderr = if matches!(stream, "stderr" | "all") {
        Some(read_log_tail(&task.stderr_log, tail_lines, max_bytes)?)
    } else {
        None
    };
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "task": task,
        "stdout": stdout,
        "stderr": stderr,
        "tail_lines": tail_lines,
    }))?)
}

/// 停止后台命令。
///
/// 参数:
/// - `args`: 工具参数
/// - `config`: 应用配置
/// - `paths`: Miyu 路径
///
/// 返回:
/// - JSON 格式停止结果
pub(super) async fn stop_background_task(
    args: Value,
    config: &AppConfig,
    paths: &MiyuPaths,
) -> Result<String> {
    let task_id = required(&args, "task_id")?;
    let force = args.get("force").and_then(Value::as_bool).unwrap_or(false);
    let store = BackgroundCommandStore::new(paths.state_dir.clone());
    let mut tasks = store.load()?;
    refresh_task_statuses(&mut tasks, config).await;
    let task = tasks
        .iter_mut()
        .find(|item| item.id == task_id)
        .ok_or_else(|| anyhow::anyhow!("background command not found: {task_id}"))?;
    let was_running = task.status == "running" && process_exists(task.pid);
    if was_running {
        terminate_process(task.pid, task.pgid, force).await;
        if !force {
            tokio::time::sleep(Duration::from_secs(
                config.tools.background_command_stop_grace_seconds,
            ))
            .await;
            if process_exists(task.pid) {
                terminate_process(task.pid, task.pgid, true).await;
            }
        }
        task.status = "stopped".to_string();
        task.updated_at = unix_seconds();
    }
    let task = task.clone();
    store.save(&tasks)?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "was_running": was_running,
        "task": task,
    }))?)
}

/// 清理已结束后台命令记录。
///
/// 参数:
/// - `args`: 工具参数
/// - `paths`: Miyu 路径
/// - `config`: 应用配置
///
/// 返回:
/// - JSON 格式清理结果
pub(super) async fn cleanup_background_tasks(
    args: Value,
    paths: &MiyuPaths,
    config: &AppConfig,
) -> Result<String> {
    let remove_logs = args
        .get("remove_logs")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let store = BackgroundCommandStore::new(paths.state_dir.clone());
    let mut tasks = store.load()?;
    refresh_task_statuses(&mut tasks, config).await;
    let mut removed = Vec::new();
    tasks.retain(|task| {
        if task.status == "running" {
            return true;
        }
        if remove_logs {
            let _ = std::fs::remove_file(&task.stdout_log);
            let _ = std::fs::remove_file(&task.stderr_log);
        }
        removed.push(task.id.clone());
        false
    });
    store.save(&tasks)?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "removed": removed,
        "remaining": tasks.len(),
    }))?)
}

/// 刷新任务运行状态。
///
/// 参数:
/// - `tasks`: 任务列表
/// - `config`: 应用配置
async fn refresh_task_statuses(tasks: &mut [BackgroundCommandTask], config: &AppConfig) {
    let now = unix_seconds();
    for task in tasks {
        if task.status != "running" {
            continue;
        }
        if !process_exists(task.pid) {
            task.status = "exited".to_string();
            task.updated_at = now;
            continue;
        }
        if !is_unlimited(task.timeout_seconds)
            && now.saturating_sub(task.started_at) >= task.timeout_seconds
        {
            terminate_process(task.pid, task.pgid, false).await;
            tokio::time::sleep(Duration::from_secs(
                config.tools.background_command_stop_grace_seconds,
            ))
            .await;
            if process_exists(task.pid) {
                terminate_process(task.pid, task.pgid, true).await;
            }
            task.status = "timed_out".to_string();
            task.updated_at = unix_seconds();
        }
    }
}

/// 查找后台任务。
///
/// 参数:
/// - `tasks`: 任务列表
/// - `task_id`: 任务 ID
///
/// 返回:
/// - 匹配任务
fn find_task<'a>(
    tasks: &'a [BackgroundCommandTask],
    task_id: &str,
) -> Result<&'a BackgroundCommandTask> {
    tasks
        .iter()
        .find(|item| item.id == task_id)
        .ok_or_else(|| anyhow::anyhow!("background command not found: {task_id}"))
}

/// 读取日志末尾若干行。
///
/// 参数:
/// - `path`: 日志路径
/// - `tail_lines`: 末尾行数
/// - `max_bytes`: 最大读取字节数
///
/// 返回:
/// - 日志文本
fn read_log_tail(path: &str, tail_lines: usize, max_bytes: u64) -> Result<String> {
    let path = Path::new(path);
    if !path.exists() {
        return Ok(String::new());
    }
    let metadata = std::fs::metadata(path)?;
    let start = metadata.len().saturating_sub(max_bytes.max(1));
    let mut file = std::fs::File::open(path)?;
    use std::io::{Read, Seek, SeekFrom};
    file.seek(SeekFrom::Start(start))?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    let lines = text.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(tail_lines);
    Ok(lines[start..].join("\n"))
}

/// 读取必填字符串参数。
///
/// 参数:
/// - `args`: 工具参数
/// - `key`: 参数名
///
/// 返回:
/// - 参数值
fn required(args: &Value, key: &str) -> Result<String> {
    let value = args
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if value.is_empty() {
        bail!("{}: {key}", t("required argument missing", "缺少必需参数"))
    } else {
        Ok(value.to_string())
    }
}

/// 展开路径。
///
/// 参数:
/// - `value`: 原始路径
///
/// 返回:
/// - 展开后的路径
fn expand_path(value: &str) -> PathBuf {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix("~/") {
        if let Some(home) = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()) {
            return home.join(rest);
        }
    }
    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

/// 生成适合任务 ID 和日志名的短标签。
///
/// 参数:
/// - `value`: 原始标签
///
/// 返回:
/// - 安全标签
fn sanitize_id(value: &str) -> String {
    let mut output = value
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else if matches!(ch, '-' | '_' | '.') {
                Some(ch)
            } else if ch.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .take(40)
        .collect::<String>();
    if output.is_empty() {
        output = "command".to_string();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_id_keeps_safe_subset() {
        assert_eq!(sanitize_id("Dev Server 01!"), "dev-server-01");
    }

    #[tokio::test]
    async fn refresh_keeps_unlimited_running_task() {
        let mut tasks = vec![BackgroundCommandTask {
            id: "task-1".to_string(),
            label: "server".to_string(),
            command: "sleep 9999".to_string(),
            cwd: ".".to_string(),
            pid: std::process::id(),
            pgid: None,
            status: "running".to_string(),
            stdout_log: "stdout.log".to_string(),
            stderr_log: "stderr.log".to_string(),
            started_at: 0,
            updated_at: 0,
            timeout_seconds: 0,
        }];

        refresh_task_statuses(&mut tasks, &AppConfig::default()).await;

        assert_eq!(tasks[0].status, "running");
    }
}
