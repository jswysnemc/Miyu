use super::event_mapper::{agent_event_to_web, encode_event, WebStreamEvent};
use super::session_store;
use crate::agent::{Agent, AgentMode};
use crate::cli::build_tool_registry;
use crate::config::AppConfig;
use crate::llm::OpenAiCompatibleClient;
use crate::paths::MiyuPaths;
use crate::state::StateStore;
use anyhow::{bail, Result};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub(crate) struct ChatRunRequest {
    pub(crate) session_id: String,
    pub(crate) message: String,
    pub(crate) image_url: Option<String>,
    pub(crate) mode: AgentMode,
    pub(crate) thinking: Option<String>,
}

/// 运行一轮 Web 对话并写入事件流。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `request`: 对话请求
/// - `tx`: 输出事件发送端
///
/// 返回:
/// - 对话运行结果
pub(crate) async fn run_chat(
    paths: MiyuPaths,
    request: ChatRunRequest,
    tx: mpsc::UnboundedSender<String>,
) -> Result<()> {
    if request.message.trim().is_empty() && request.image_url.is_none() {
        bail!("message is required");
    }
    session_store::ensure_session(&paths, &request.session_id)?;
    send_event(
        &tx,
        WebStreamEvent::Session {
            session_id: request.session_id.clone(),
        },
    );
    AppConfig::init_files(&paths)?;
    let mut config = AppConfig::load_or_default(&paths)?;
    apply_thinking_override(&mut config, request.thinking.as_deref())?;
    let session_paths = session_store::session_paths(&paths, &request.session_id);
    let state = StateStore::new(&session_paths)?;
    state.init_files()?;
    let client = OpenAiCompatibleClient::from_config(&config, &paths)?;
    let registry = build_tool_registry(&config, &paths, request.mode)?;
    let progressive_loading_enabled = config.tools.progressive_loading_enabled;
    let mut agent = Agent::new(
        config,
        &session_paths,
        state.clone(),
        client,
        registry,
        request.mode,
    )?;
    if progressive_loading_enabled {
        let loaded_tools = state.load_loaded_tools()?;
        agent.restore_loaded_tools(&loaded_tools);
    }
    let result = agent
        .chat_stream_with_image(&request.message, request.image_url, |event| {
            send_event(&tx, agent_event_to_web(event));
            Ok(())
        })
        .await?;
    let loaded_tools = if progressive_loading_enabled {
        let loaded_tools = agent.loaded_tools();
        state.save_loaded_tools(&loaded_tools)?;
        loaded_tools
    } else {
        Vec::new()
    };
    session_store::touch_session_with_message(&paths, &request.session_id, &request.message)?;
    send_event(
        &tx,
        WebStreamEvent::Done {
            content: result.content,
            reasoning: result.reasoning,
            usage: result.usage,
            loaded_tools,
        },
    );
    Ok(())
}

/// 发送 Web 流事件。
///
/// 参数:
/// - `tx`: 输出事件发送端
/// - `event`: Web 流事件
///
/// 返回:
/// - 无
fn send_event(tx: &mpsc::UnboundedSender<String>, event: WebStreamEvent) {
    let _ = tx.send(encode_event(&event));
}

/// 应用临时思考等级覆盖。
///
/// 参数:
/// - `config`: 应用配置
/// - `level`: 思考等级
///
/// 返回:
/// - 应用是否成功
fn apply_thinking_override(config: &mut AppConfig, level: Option<&str>) -> Result<()> {
    let Some(level) = level.map(str::trim).filter(|level| !level.is_empty()) else {
        return Ok(());
    };
    match level {
        "auto" | "none" | "low" | "medium" | "high" | "xhigh" | "max" => {}
        value => bail!("invalid thinking level: {value}"),
    }
    let active = config.active_provider.clone();
    let provider = config
        .providers
        .iter_mut()
        .find(|provider| provider.id == active)
        .ok_or_else(|| anyhow::anyhow!("provider not found: {active}"))?;
    provider.thinking_level = level.to_string();
    Ok(())
}
