use super::subagent_runner::{ProgressMode, SubagentProgress, SubagentRunner, SubagentStats};
use super::{task_runtime, task_state, ToolProgress, ToolRegistry, ToolSpec};
use crate::config::AppConfig;
use crate::i18n::text as t;
use crate::llm::OpenAiCompatibleClient;
use crate::paths::MiyuPaths;
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::time::Duration;

const EXPLORE_PROMPT: &str = include_str!("../prompts/subagent-explore.md");
const GENERAL_PROMPT: &str = include_str!("../prompts/subagent-general.md");
const DEFAULT_SUBAGENT_TYPE: &str = "general";
const DEFAULT_MAX_STEPS: usize = 20;
const MAX_MAX_STEPS: usize = 80;
const TASK_TIMEOUT_SECONDS: u64 = 1800;
const TOOL_TIMEOUT_SECONDS: u64 = 120;
const DESCRIPTION_MAX_CHARS: usize = 160;

const EXPLORE_ALLOWED: &[&str] = &[
    "check_os_info",
    "read_file",
    "glob",
    "find_files",
    "grep",
    "search_text",
    "web_search",
    "web_fetch",
];

const GENERAL_EXCLUDED: &[&str] = &[
    "task",
    "background_command",
    "deep_research",
    "deep_diagnose",
    "linux_input_method_diagnose",
    "linux_game_compatibility",
    "load",
    "set_alarm",
    "list_alarms",
    "cancel_alarm",
    "search_meme",
    "show_meme",
    "add_meme",
    "update_meme",
    "delete_meme",
    "generate_image",
    "search_web_images",
    "xuanxue_pick",
    "xuanxue_divine",
    "draw_zhouyi_hexagram",
    "draw_tarot_card",
    "draw_fortune_lot",
    "roll_dice",
];

#[derive(Clone)]
struct TaskContext {
    config: AppConfig,
    paths: MiyuPaths,
    tools: ToolRegistry,
}

/// 注册 REPL 子代理任务工具。
///
/// 参数:
/// - `registry`: 工具注册表
/// - `config`: 应用配置
/// - `paths`: Miyu 路径
/// - `tools`: 子代理可用工具注册表
///
/// 返回:
/// - 无
pub(crate) fn register(
    registry: &mut ToolRegistry,
    config: AppConfig,
    paths: MiyuPaths,
    tools: ToolRegistry,
) {
    let context = TaskContext {
        config,
        paths,
        tools,
    };
    registry.register(ToolSpec::new_with_progress(
        "task",
        t(
            "Start and manage an in-process background subagent task. Only available in interactive REPL sessions. Use action=start to run without blocking the main conversation; then poll status or result by task_id.",
            "启动并管理进程内后台子代理任务。此工具只在交互式 REPL 会话中可用。使用 action=start 后不会阻塞主对话；随后用 task_id 查询状态或结果。",
        ),
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["start", "status", "result", "list", "cancel"],
                    "description": t("Operation to perform. Defaults to start.", "要执行的操作，默认 start。")
                },
                "description": {
                    "type": "string",
                    "description": t("Short task label for display when starting a task.", "启动任务时展示用的短描述。")
                },
                "prompt": {
                    "type": "string",
                    "description": t("Full instruction for the subagent when starting a task.", "启动任务时交给子代理的完整指令。")
                },
                "subagent_type": {
                    "type": "string",
                    "enum": ["general", "explore"],
                    "description": t("Subagent type. general can use normal tools; explore is read/search focused.", "子代理类型。general 可使用常规工具；explore 偏向只读搜索。")
                },
                "max_steps": {
                    "type": "integer",
                    "description": t("Maximum tool calls for the subagent. Defaults to 20.", "子代理最大工具调用次数，默认 20。")
                },
                "task_id": {
                    "type": "string",
                    "description": t("Task id for status, result, or cancel.", "status、result 或 cancel 使用的任务 ID。")
                }
            },
            "additionalProperties": false
        }),
        move |args, _progress| {
            let context = context.clone();
            async move { run_task(args, context).await }
        },
    ));
}

/// 分发后台子代理任务操作。
///
/// 参数:
/// - `args`: 工具参数
/// - `context`: 任务上下文
///
/// 返回:
/// - JSON 字符串形式的操作结果
async fn run_task(args: Value, context: TaskContext) -> Result<String> {
    let action = optional_string_arg(&args, "action")?.unwrap_or_else(|| "start".to_string());
    match action.as_str() {
        "start" => start_task(args, context).await,
        "status" => task_status(args),
        "result" => task_result(args),
        "list" => task_list(),
        "cancel" => task_cancel(args),
        _ => bail!("unsupported task action: {action}"),
    }
}

/// 启动后台子代理任务。
///
/// 参数:
/// - `args`: 启动参数
/// - `context`: 任务上下文
///
/// 返回:
/// - 已创建任务的快照
async fn start_task(args: Value, context: TaskContext) -> Result<String> {
    let prompt = string_arg(&args, "prompt")?;
    let description = optional_string_arg(&args, "description")?
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| summarize_prompt(&prompt));
    let subagent_type = optional_string_arg(&args, "subagent_type")?
        .unwrap_or_else(|| DEFAULT_SUBAGENT_TYPE.into());
    if !matches!(subagent_type.as_str(), "general" | "explore") {
        bail!("unsupported subagent_type: {subagent_type}");
    }
    let max_steps = args
        .get("max_steps")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(DEFAULT_MAX_STEPS)
        .clamp(1, MAX_MAX_STEPS);
    let (task, cancel_rx) = task_state::create_task(description, subagent_type, max_steps);
    let _ = task_runtime::record_subagent_task_started(&context.paths, &task);
    let task_id = task.id.clone();
    tokio::spawn(async move {
        execute_subagent_task(task_id, prompt, context, cancel_rx).await;
    });
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "task": task,
        "message": t(
            "subagent task started; use action=status or action=result with task_id",
            "子代理任务已启动；使用 action=status 或 action=result 配合 task_id 查询"
        )
    }))?)
}

/// 执行后台子代理任务并写回任务状态。
///
/// 参数:
/// - `task_id`: 任务 ID
/// - `prompt`: 子代理任务提示
/// - `context`: 任务上下文
/// - `cancel_rx`: 取消信号接收器
///
/// 返回:
/// - 无
async fn execute_subagent_task(
    task_id: String,
    prompt: String,
    context: TaskContext,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let paths = context.paths.clone();
    let task = match task_state::task_snapshot(&task_id) {
        Ok(task) => task,
        Err(err) => {
            task_state::finish_task(&task_id, "failed", None, Some(err.to_string()), None);
            record_finished_runtime_task(&paths, &task_id);
            return;
        }
    };
    let result = tokio::select! {
        _ = &mut cancel_rx => Err(anyhow::anyhow!("cancelled")),
        result = run_subagent(&task.subagent_type, task.max_steps, &prompt, context) => result,
    };
    match result {
        Ok((content, stats)) => {
            task_state::finish_task(&task_id, "completed", Some(content), None, Some(stats));
            record_finished_runtime_task(&paths, &task_id);
        }
        Err(err) if err.to_string() == "cancelled" => {
            task_state::finish_task(
                &task_id,
                "cancelled",
                None,
                Some("cancelled".to_string()),
                None,
            );
            record_finished_runtime_task(&paths, &task_id);
        }
        Err(err) => {
            task_state::finish_task(&task_id, "failed", None, Some(err.to_string()), None);
            record_finished_runtime_task(&paths, &task_id);
        }
    }
}

/// 记录已结束子代理任务的运行时状态。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `task_id`: 子代理任务 ID
///
/// 返回:
/// - 无
fn record_finished_runtime_task(paths: &MiyuPaths, task_id: &str) {
    if let Ok(task) = task_state::task_snapshot(task_id) {
        let _ = task_runtime::record_subagent_task_finished(paths, &task);
    }
}

/// 运行指定类型的子代理。
///
/// 参数:
/// - `subagent_type`: 子代理类型
/// - `max_steps`: 最大工具调用次数
/// - `prompt`: 子代理任务提示
/// - `context`: 任务上下文
///
/// 返回:
/// - 子代理输出内容和公开统计信息
async fn run_subagent(
    subagent_type: &str,
    max_steps: usize,
    prompt: &str,
    context: TaskContext,
) -> Result<(String, Value)> {
    let client = OpenAiCompatibleClient::from_config(&context.config, &context.paths)?;
    let (system_prompt, tools, excluded) = match subagent_type {
        "explore" => (
            EXPLORE_PROMPT,
            context.tools.clone_filtered(EXPLORE_ALLOWED),
            Vec::new(),
        ),
        "general" => (GENERAL_PROMPT, context.tools, GENERAL_EXCLUDED.to_vec()),
        _ => bail!("unsupported subagent_type: {subagent_type}"),
    };
    let progress = SubagentProgress::new(
        ToolProgress::default(),
        ProgressMode::from_config(&context.config),
        false,
    );
    let runner = SubagentRunner::new(client, system_prompt, tools, progress)
        .max_steps(max_steps)
        .timeout_seconds(TOOL_TIMEOUT_SECONDS)
        .excluded_tools(&excluded);
    let result = tokio::time::timeout(
        Duration::from_secs(TASK_TIMEOUT_SECONDS),
        runner.run(prompt),
    )
    .await
    .map_err(|_| anyhow::anyhow!("subagent task timed out after {TASK_TIMEOUT_SECONDS}s"))??;
    let (chat_result, stats) = result;
    Ok((chat_result.content, stats_json(&stats)))
}

/// 生成子代理任务统计 JSON。
///
/// 参数:
/// - `stats`: 子代理统计
///
/// 返回:
/// - 公开统计信息
fn stats_json(stats: &SubagentStats) -> Value {
    let mut value = stats.public();
    if let Value::Object(map) = &mut value {
        map.insert("budget_reached".to_string(), json!(stats.budget_reached));
    }
    value
}

/// 查询单个后台子代理任务状态。
///
/// 参数:
/// - `args`: 查询参数
///
/// 返回:
/// - 任务快照
fn task_status(args: Value) -> Result<String> {
    let task_id = string_arg(&args, "task_id")?;
    let task = task_state::task_snapshot(&task_id)?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "task": task
    }))?)
}

/// 查询后台子代理任务结果。
///
/// 参数:
/// - `args`: 查询参数
///
/// 返回:
/// - 任务结果或当前状态
fn task_result(args: Value) -> Result<String> {
    let task_id = string_arg(&args, "task_id")?;
    let task = task_state::task_snapshot(&task_id)?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": task.status == "completed",
        "task": task
    }))?)
}

/// 列出后台子代理任务。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 任务列表
fn task_list() -> Result<String> {
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "tasks": task_state::list_tasks()
    }))?)
}

/// 取消后台子代理任务。
///
/// 参数:
/// - `args`: 取消参数
///
/// 返回:
/// - 取消后的任务快照
fn task_cancel(args: Value) -> Result<String> {
    let task_id = string_arg(&args, "task_id")?;
    let task = task_state::cancel_task(&task_id)?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "task": task
    }))?)
}

/// 读取可选字符串参数。
///
/// 参数:
/// - `args`: JSON 参数
/// - `name`: 参数名称
///
/// 返回:
/// - 可选字符串
fn optional_string_arg(args: &Value, name: &str) -> Result<Option<String>> {
    let Some(value) = args.get(name) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(text) = value.as_str() else {
        bail!("{name} must be a string");
    };
    Ok(Some(text.trim().to_string()))
}

/// 读取必填字符串参数。
///
/// 参数:
/// - `args`: JSON 参数
/// - `name`: 参数名称
///
/// 返回:
/// - 字符串参数
fn string_arg(args: &Value, name: &str) -> Result<String> {
    let value = optional_string_arg(args, name)?;
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        bail!("missing required string argument: {name}");
    };
    Ok(value)
}

/// 从任务提示中生成短描述。
///
/// 参数:
/// - `prompt`: 子代理任务提示
///
/// 返回:
/// - 短描述文本
fn summarize_prompt(prompt: &str) -> String {
    let mut description = prompt
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(DESCRIPTION_MAX_CHARS)
        .collect::<String>();
    if description.is_empty() {
        description = "subagent task".to_string();
    }
    description
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_action_defaults_to_none() {
        let value = json!({});

        assert_eq!(optional_string_arg(&value, "action").unwrap(), None);
    }

    #[test]
    fn rejects_non_string_optional_argument() {
        let value = json!({"action": 123});

        assert!(optional_string_arg(&value, "action").is_err());
    }

    #[test]
    fn reads_required_string_argument() {
        let value = json!({"prompt": " inspect code "});

        assert_eq!(string_arg(&value, "prompt").unwrap(), "inspect code");
    }

    #[test]
    fn rejects_empty_required_string_argument() {
        let value = json!({"prompt": "   "});

        assert!(string_arg(&value, "prompt").is_err());
    }

    #[test]
    fn summarizes_prompt_text() {
        assert_eq!(
            summarize_prompt("  inspect   this code\nnow "),
            "inspect this code now"
        );
    }
}
