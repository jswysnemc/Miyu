use super::subagent_runner::{ProgressMode, SubagentProgress, SubagentRunner, SubagentStats};
use super::{subagent_runtime, subagent_state, ToolProgress, ToolRegistry, ToolSpec};
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
const SUBAGENT_TIMEOUT_SECONDS: u64 = 1800;
const TOOL_TIMEOUT_SECONDS: u64 = 120;
const DESCRIPTION_MAX_CHARS: usize = 160;

const EXPLORE_ALLOWED: &[&str] = &[
    "check_os_info",
    "read_file",
    "glob",
    "grep",
    "web_search",
    "web_fetch",
];

const GENERAL_EXCLUDED: &[&str] = &[
    "subagent",
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
struct SubagentContext {
    config: AppConfig,
    paths: MiyuPaths,
    tools: ToolRegistry,
}

/// 注册交互式会话子智能体工具。
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
    let context = SubagentContext {
        config,
        paths,
        tools,
    };
    registry.register(ToolSpec::new_with_progress(
        "subagent",
        t(
            "Start and manage an in-process subagent. Only available in interactive REPL and Web sessions. Use action=start to run without blocking the main conversation. Do not poll action=status in a loop: when a subagent finishes you receive an automatic system-reminder, then call action=result with its subagent_id to collect the output. action=list shows all subagents; action=cancel stops one.",
            "启动并管理进程内子智能体。此工具只在交互式 REPL 和 Web 会话中可用。使用 action=start 后不会阻塞主对话。不要循环调用 action=status 轮询：子智能体完成时你会收到自动的系统提醒,届时再用 action=result 配合 subagent_id 取回结果。action=list 列出全部子智能体；action=cancel 取消某个。",
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
                    "description": t("Short label for display when starting a subagent.", "启动子智能体时展示用的短描述。")
                },
                "prompt": {
                    "type": "string",
                    "description": t("Full instruction for the subagent.", "交给子智能体的完整指令。")
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
                "subagent_id": {
                    "type": "string",
                    "description": t("Subagent id for status, result, or cancel.", "status、result 或 cancel 使用的子智能体 ID。")
                }
            },
            "additionalProperties": false
        }),
        move |args, _progress| {
            let context = context.clone();
            async move { run_subagent_action(args, context).await }
        },
    ));
}

/// 分发子智能体操作。
///
/// 参数:
/// - `args`: 工具参数
/// - `context`: 子智能体上下文
///
/// 返回:
/// - JSON 字符串形式的操作结果
async fn run_subagent_action(args: Value, context: SubagentContext) -> Result<String> {
    let action = optional_string_arg(&args, "action")?.unwrap_or_else(|| "start".to_string());
    match action.as_str() {
        "start" => start_subagent(args, context).await,
        "status" => subagent_status(args),
        "result" => subagent_result(args),
        "list" => subagent_list(),
        "cancel" => subagent_cancel(args),
        _ => bail!("unsupported subagent action: {action}"),
    }
}

/// 启动后台子智能体。
///
/// 参数:
/// - `args`: 启动参数
/// - `context`: 子智能体上下文
///
/// 返回:
/// - 已创建子智能体的快照
async fn start_subagent(args: Value, context: SubagentContext) -> Result<String> {
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
    let (subagent, cancel_rx) =
        subagent_state::create_subagent(description, subagent_type, max_steps);
    let _ = subagent_runtime::record_subagent_started(&context.paths, &subagent);
    let subagent_id = subagent.id.clone();
    tokio::spawn(async move {
        execute_subagent(subagent_id, prompt, context, cancel_rx).await;
    });
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "subagent": subagent,
        "message": t(
            "subagent started; use action=status or action=result with subagent_id",
            "子智能体已启动；使用 action=status 或 action=result 配合 subagent_id 查询"
        )
    }))?)
}

/// 执行后台子智能体并写回状态。
///
/// 参数:
/// - `subagent_id`: 子智能体 ID
/// - `prompt`: 子智能体提示
/// - `context`: 子智能体上下文
/// - `cancel_rx`: 取消信号接收器
///
/// 返回:
/// - 无
async fn execute_subagent(
    subagent_id: String,
    prompt: String,
    context: SubagentContext,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let paths = context.paths.clone();
    let subagent = match subagent_state::subagent_snapshot(&subagent_id) {
        Ok(subagent) => subagent,
        Err(err) => {
            subagent_state::finish_subagent(
                &subagent_id,
                "failed",
                None,
                Some(err.to_string()),
                None,
            );
            record_finished_runtime_subagent(&paths, &subagent_id);
            return;
        }
    };
    // 1. 起进度 channel，把子代理的阶段/工具上报实时写回快照，供前端轮询感知
    let (progress_tx, progress_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let progress_task = tokio::spawn(consume_subagent_progress(subagent_id.clone(), progress_rx));
    let progress = ToolProgress::new(progress_tx);
    let result = tokio::select! {
        _ = &mut cancel_rx => Err(anyhow::anyhow!("cancelled")),
        result = run_subagent(
            &subagent.subagent_type,
            subagent.max_steps,
            &prompt,
            context,
            progress,
        ) => result,
    };
    // 2. 关闭 sender 后等待消费任务收尾，确保进度写入不丢
    progress_task.abort();
    match result {
        Ok((content, stats)) => {
            subagent_state::finish_subagent(
                &subagent_id,
                "completed",
                Some(content),
                None,
                Some(stats),
            );
            record_finished_runtime_subagent(&paths, &subagent_id);
        }
        Err(err) if err.to_string() == "cancelled" => {
            subagent_state::finish_subagent(
                &subagent_id,
                "cancelled",
                None,
                Some("cancelled".to_string()),
                None,
            );
            record_finished_runtime_subagent(&paths, &subagent_id);
        }
        Err(err) => {
            subagent_state::finish_subagent(
                &subagent_id,
                "failed",
                None,
                Some(err.to_string()),
                None,
            );
            record_finished_runtime_subagent(&paths, &subagent_id);
        }
    }
}

/// 消费子代理进度消息并写回快照。
///
/// 参数:
/// - `subagent_id`: 子智能体 ID
/// - `progress_rx`: 进度消息接收器
///
/// 返回:
/// - 无
async fn consume_subagent_progress(
    subagent_id: String,
    mut progress_rx: tokio::sync::mpsc::UnboundedReceiver<String>,
) {
    while let Some(message) = progress_rx.recv().await {
        // 1. reasoning 与 full 模式的结构化消息不作为面板进度，跳过
        if message.starts_with("__subagent_reasoning__")
            || message.starts_with("__subtool_call__")
            || message.starts_with("__subtool_result__")
        {
            continue;
        }
        subagent_state::update_subagent_progress(&subagent_id, parse_progress_message(&message));
    }
}

/// 从进度文本解析出结构化进度更新。
///
/// 参数:
/// - `message`: 子代理上报的进度文本
///
/// 返回:
/// - 结构化进度更新
fn parse_progress_message(message: &str) -> subagent_state::SubagentProgressUpdate {
    let mut update = subagent_state::SubagentProgressUpdate {
        phase: Some(message.to_string()),
        ..Default::default()
    };
    // 1. 识别 "工具 #N：名称 ..." 或 "tool #N: name ..." 形式，提取步数与工具名
    let marker = message.find('#');
    if let Some(marker) = marker {
        let rest = &message[marker + 1..];
        let digits = rest
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>();
        if let Ok(step) = digits.parse::<usize>() {
            update.step = Some(step);
            let after = rest[digits.len()..]
                .trim_start_matches([':', '：', ' '])
                .trim();
            let name = after
                .split([' ', '：', ':'])
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !name.is_empty() {
                update.last_tool = Some(name);
            }
        }
    }
    update
}

/// 记录已结束子智能体的运行时状态。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `subagent_id`: 子智能体 ID
///
/// 返回:
/// - 无
fn record_finished_runtime_subagent(paths: &MiyuPaths, subagent_id: &str) {
    if let Ok(subagent) = subagent_state::subagent_snapshot(subagent_id) {
        let _ = subagent_runtime::record_subagent_finished(paths, &subagent);
    }
}

/// 运行指定类型的子代理。
///
/// 参数:
/// - `subagent_type`: 子代理类型
/// - `max_steps`: 最大工具调用次数
/// - `prompt`: 子智能体提示
/// - `context`: 子智能体上下文
/// - `tool_progress`: 写回快照的进度上报通道
///
/// 返回:
/// - 子代理输出内容和公开统计信息
async fn run_subagent(
    subagent_type: &str,
    max_steps: usize,
    prompt: &str,
    context: SubagentContext,
    tool_progress: ToolProgress,
) -> Result<(String, Value)> {
    // 1. 按子智能体模型配置构造客户端,未配置时沿用主对话供应商与模型
    let client = build_subagent_client(&context)?;
    let (system_prompt, tools, excluded) = match subagent_type {
        "explore" => (
            EXPLORE_PROMPT,
            context.tools.clone_filtered(EXPLORE_ALLOWED),
            Vec::new(),
        ),
        "general" => (GENERAL_PROMPT, context.tools, GENERAL_EXCLUDED.to_vec()),
        _ => bail!("unsupported subagent_type: {subagent_type}"),
    };
    // 1. 强制以 Summary 模式上报，让面板能看到工具步进而非依赖全局展示配置
    let progress = SubagentProgress::new(tool_progress, ProgressMode::Summary, true);
    let runner = SubagentRunner::new(client, system_prompt, tools, progress)
        .max_steps(max_steps)
        .timeout_seconds(TOOL_TIMEOUT_SECONDS)
        .excluded_tools(&excluded);
    let result = tokio::time::timeout(
        Duration::from_secs(SUBAGENT_TIMEOUT_SECONDS),
        runner.run(prompt),
    )
    .await
    .map_err(|_| anyhow::anyhow!("subagent timed out after {SUBAGENT_TIMEOUT_SECONDS}s"))??;
    let (chat_result, stats) = result;
    Ok((chat_result.content, stats_json(&stats)))
}

/// 按子智能体模型配置构造 LLM 客户端。
///
/// 子智能体配置了独立供应商/模型时,在一份克隆配置上覆盖 active_provider 与该供应商
/// 的 default_model;未配置时直接沿用主对话配置。
///
/// 参数:
/// - `context`: 子智能体上下文
///
/// 返回:
/// - LLM 客户端
fn build_subagent_client(context: &SubagentContext) -> Result<OpenAiCompatibleClient> {
    let subagent = &context.config.subagent;
    // 1. 未配置任何子智能体供应商与模型,沿用主对话配置
    if subagent.provider_id.is_empty() && subagent.model.is_empty() {
        return OpenAiCompatibleClient::from_config(&context.config, &context.paths);
    }
    let mut config = context.config.clone();
    // 2. 指定了供应商则切换 active_provider,否则在当前供应商上改模型
    if !subagent.provider_id.is_empty() {
        config.active_provider = subagent.provider_id.clone();
    }
    if !subagent.model.is_empty() {
        let active = config.active_provider.clone();
        if let Some(provider) = config
            .providers
            .iter_mut()
            .find(|provider| provider.id == active)
        {
            provider.default_model = subagent.model.clone();
        }
    }
    OpenAiCompatibleClient::from_config(&config, &context.paths)
}

/// 生成子智能体统计 JSON。
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

/// 查询单个后台子智能体状态。
///
/// 参数:
/// - `args`: 查询参数
///
/// 返回:
/// - 子智能体快照
fn subagent_status(args: Value) -> Result<String> {
    let subagent_id = string_arg(&args, "subagent_id")?;
    let subagent = subagent_state::subagent_snapshot(&subagent_id)?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "subagent": subagent
    }))?)
}

/// 查询后台子智能体结果。
///
/// 参数:
/// - `args`: 查询参数
///
/// 返回:
/// - 子智能体结果或当前状态
fn subagent_result(args: Value) -> Result<String> {
    let subagent_id = string_arg(&args, "subagent_id")?;
    let subagent = subagent_state::subagent_snapshot(&subagent_id)?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": subagent.status == "completed",
        "subagent": subagent
    }))?)
}

/// 列出后台子智能体。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 子智能体列表
fn subagent_list() -> Result<String> {
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "subagents": subagent_state::list_subagents()
    }))?)
}

/// 取消后台子智能体。
///
/// 参数:
/// - `args`: 取消参数
///
/// 返回:
/// - 取消后的子智能体快照
fn subagent_cancel(args: Value) -> Result<String> {
    let subagent_id = string_arg(&args, "subagent_id")?;
    let subagent = subagent_state::cancel_subagent(&subagent_id)?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "subagent": subagent
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

/// 从子智能体提示中生成短描述。
///
/// 参数:
/// - `prompt`: 子智能体提示
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
        description = "subagent".to_string();
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

    #[test]
    fn parses_chinese_tool_progress() {
        let update = parse_progress_message("工具 #3：Shell 运行中");

        assert_eq!(update.step, Some(3));
        assert_eq!(update.last_tool.as_deref(), Some("Shell"));
        assert_eq!(update.phase.as_deref(), Some("工具 #3：Shell 运行中"));
    }

    #[test]
    fn parses_english_tool_progress() {
        let update = parse_progress_message("tool #12: read_file running");

        assert_eq!(update.step, Some(12));
        assert_eq!(update.last_tool.as_deref(), Some("read_file"));
    }

    #[test]
    fn keeps_plain_phase_without_step() {
        let update = parse_progress_message("工具调用 5 次　消耗 Token 1.2K");

        assert_eq!(update.step, None);
        assert_eq!(update.last_tool, None);
        assert_eq!(
            update.phase.as_deref(),
            Some("工具调用 5 次　消耗 Token 1.2K")
        );
    }
}
