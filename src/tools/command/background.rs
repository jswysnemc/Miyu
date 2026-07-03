use super::process::{process_exists, spawn_background_shell, terminate_process};
use super::store::{unix_seconds, BackgroundCommandStore, BackgroundCommandTask};
use crate::config::AppConfig;
use crate::i18n::text as t;
use crate::paths::MiyuPaths;
use crate::tools::{ToolRegistry, ToolSpec};
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// 注册后台命令写入类工具。
///
/// 参数:
/// - `registry`: 工具注册表
/// - `config`: 应用配置
/// - `paths`: Miyu 路径
/// - `allow_command_execution`: 是否允许命令执行
pub(crate) fn register(
    registry: &mut ToolRegistry,
    config: AppConfig,
    paths: MiyuPaths,
    allow_command_execution: bool,
) {
    let start_config = config.clone();
    let start_paths = paths.clone();
    registry.register(ToolSpec::new(
        "start_background_command",
        t("Start a long-running shell command as a managed background task. Returns task_id, pid, and log paths.", "把长时间运行的 shell 命令作为可管理后台任务启动，返回 task_id、pid 和日志路径。"),
        json!({"type":"object","properties":{"command":{"type":"string","description":t("Command to run in background.","后台运行的命令。")},"cwd":{"type":"string","description":t("Optional working directory. Defaults to current workspace.","可选工作目录，默认当前工作区。")},"label":{"type":"string","description":t("Optional human-readable label.","可选的人类可读标签。")},"timeout_seconds":{"type":"integer","description":t("Optional task timeout metadata, max configured default.","可选任务超时元数据，最大不超过配置默认值。")}},"required":["command"],"additionalProperties":false}),
        move |args| {
            let config = start_config.clone();
            let paths = start_paths.clone();
            async move { start_background_command(args, &config, &paths, allow_command_execution) }
        },
    ).writes());

    register_readonly(registry, config.clone(), paths.clone());

    let stop_config = config.clone();
    let stop_paths = paths.clone();
    registry.register(ToolSpec::new(
        "stop_background_command",
        t("Stop a managed background command by task_id. Sends TERM first, then KILL after the configured grace period.", "按 task_id 停止受管理的后台命令。先发送 TERM，超过配置宽限时间后发送 KILL。"),
        json!({"type":"object","properties":{"task_id":{"type":"string","description":t("Background task id.","后台任务 ID。")},"force":{"type":"boolean","description":t("Force kill immediately.","立即强制终止。")}},"required":["task_id"],"additionalProperties":false}),
        move |args| {
            let config = stop_config.clone();
            let paths = stop_paths.clone();
            async move { stop_background_command(args, &config, &paths).await }
        },
    ).writes());

    let cleanup_config = config.clone();
    let cleanup_paths = paths.clone();
    registry.register(ToolSpec::new(
        "cleanup_background_commands",
        t("Remove finished background command records and old logs. Running tasks are kept.", "清理已结束后台命令记录和旧日志，运行中的任务会保留。"),
        json!({"type":"object","properties":{"remove_logs":{"type":"boolean","description":t("Whether to remove logs for cleaned tasks.","是否删除被清理任务的日志。")}},"additionalProperties":false}),
        move |args| {
            let config = cleanup_config.clone();
            let paths = cleanup_paths.clone();
            async move { cleanup_background_commands(args, &paths, &config).await }
        },
    ).writes());
}

/// 注册后台命令只读工具。
///
/// 参数:
/// - `registry`: 工具注册表
/// - `config`: 应用配置
/// - `paths`: Miyu 路径
pub(crate) fn register_readonly(registry: &mut ToolRegistry, config: AppConfig, paths: MiyuPaths) {
    let list_config = config.clone();
    let list_paths = paths.clone();
    registry.register(ToolSpec::new(
        "list_background_commands",
        t(
            "List managed background commands and refresh their running status.",
            "列出受管理的后台命令并刷新运行状态。",
        ),
        json!({"type":"object","properties":{},"additionalProperties":false}),
        move |_| {
            let config = list_config.clone();
            let paths = list_paths.clone();
            async move { list_background_commands(&paths, &config).await }
        },
    ));

    let output_config = config.clone();
    let output_paths = paths.clone();
    registry.register(ToolSpec::new(
        "read_background_command_output",
        t("Read stdout/stderr logs for a managed background command by task_id.", "按 task_id 读取受管理后台命令的 stdout/stderr 日志。"),
        json!({"type":"object","properties":{"task_id":{"type":"string","description":t("Background task id.","后台任务 ID。")},"stream":{"type":"string","enum":["stdout","stderr","all"],"description":t("Which log stream to read. Defaults to all.","读取哪个日志流，默认 all。")},"tail_lines":{"type":"integer","description":t("Number of last lines to return. Defaults to 200, max 2000.","返回末尾行数，默认 200，最大 2000。")}},"required":["task_id"],"additionalProperties":false}),
        move |args| {
            let config = output_config.clone();
            let paths = output_paths.clone();
            async move { read_background_command_output(args, &config, &paths).await }
        },
    ));
}

/// 用户 CLI 列出后台命令。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - JSON 格式任务列表
pub(crate) async fn list_background_commands_for_user(
    paths: &MiyuPaths,
    config: &AppConfig,
) -> Result<String> {
    list_background_commands(paths, config).await
}

/// 用户 CLI 启动后台命令。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `config`: 应用配置
/// - `command`: shell 命令
/// - `cwd`: 可选工作目录
/// - `label`: 可选标签
/// - `timeout_seconds`: 可选超时元数据
///
/// 返回:
/// - JSON 格式任务信息
pub(crate) fn start_background_command_for_user(
    paths: &MiyuPaths,
    config: &AppConfig,
    command: &str,
    cwd: Option<&str>,
    label: Option<&str>,
    timeout_seconds: Option<u64>,
) -> Result<String> {
    let mut args = json!({"command": command});
    if let Some(cwd) = cwd.filter(|value| !value.trim().is_empty()) {
        args["cwd"] = json!(cwd);
    }
    if let Some(label) = label.filter(|value| !value.trim().is_empty()) {
        args["label"] = json!(label);
    }
    if let Some(timeout_seconds) = timeout_seconds {
        args["timeout_seconds"] = json!(timeout_seconds);
    }
    start_background_command(args, config, paths, true)
}

/// 用户 CLI 读取后台命令输出。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `config`: 应用配置
/// - `task_id`: 后台任务 ID
/// - `stream`: 输出流
/// - `tail_lines`: 读取末尾行数
///
/// 返回:
/// - JSON 格式输出
pub(crate) async fn read_background_command_output_for_user(
    paths: &MiyuPaths,
    config: &AppConfig,
    task_id: &str,
    stream: &str,
    tail_lines: usize,
) -> Result<String> {
    read_background_command_output(
        json!({"task_id": task_id, "stream": stream, "tail_lines": tail_lines}),
        config,
        paths,
    )
    .await
}

/// 用户 CLI 停止后台命令。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `config`: 应用配置
/// - `task_id`: 后台任务 ID
/// - `force`: 是否强制停止
///
/// 返回:
/// - JSON 格式停止结果
pub(crate) async fn stop_background_command_for_user(
    paths: &MiyuPaths,
    config: &AppConfig,
    task_id: &str,
    force: bool,
) -> Result<String> {
    stop_background_command(json!({"task_id": task_id, "force": force}), config, paths).await
}

/// 用户 CLI 清理后台命令记录。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `config`: 应用配置
/// - `remove_logs`: 是否删除日志
///
/// 返回:
/// - JSON 格式清理结果
pub(crate) async fn cleanup_background_commands_for_user(
    paths: &MiyuPaths,
    config: &AppConfig,
    remove_logs: bool,
) -> Result<String> {
    cleanup_background_commands(json!({"remove_logs": remove_logs}), paths, config).await
}

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
fn start_background_command(
    args: Value,
    config: &AppConfig,
    paths: &MiyuPaths,
    allowed: bool,
) -> Result<String> {
    if !allowed {
        bail!("{}", t("command execution is disabled; set skills.allow_command_execution=true in config.jsonc to enable background commands", "命令执行已禁用；请在 config.jsonc 中设置 skills.allow_command_execution=true 以启用后台命令"));
    }
    if !config.tools.background_commands_enabled {
        bail!("background commands are disabled");
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
            "background command cwd is not a directory: {}",
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
    let timeout_seconds = args
        .get("timeout_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(config.tools.background_command_timeout_seconds)
        .clamp(1, config.tools.background_command_timeout_seconds.max(1));
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
        "note": "Use list_background_commands, read_background_command_output, and stop_background_command to manage this task."
    }))?)
}

/// 列出后台命令。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - JSON 格式任务列表
async fn list_background_commands(paths: &MiyuPaths, config: &AppConfig) -> Result<String> {
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
async fn read_background_command_output(
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
async fn stop_background_command(
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
///
/// 返回:
/// - JSON 格式清理结果
async fn cleanup_background_commands(
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
        if task.timeout_seconds > 0 && now.saturating_sub(task.started_at) >= task.timeout_seconds {
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
}
