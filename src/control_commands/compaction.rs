use crate::agent::{Agent, AgentMode};
use crate::config::AppConfig;
use crate::i18n::text as t;
use crate::llm::OpenAiCompatibleClient;
use crate::paths::MiyuPaths;
use crate::state::StateStore;
use crate::tools::ToolRegistry;
use anyhow::Result;

/// 使用现有 Agent 依赖手动压缩当前会话。
///
/// 参数:
/// - `config`: 当前配置
/// - `paths`: Miyu 路径
/// - `state`: 当前状态存储
/// - `client`: 当前 LLM 客户端
/// - `mode`: 当前 Agent 模式
/// - `keep_tail_turns`: 保留的最近非运行轮次数量
///
/// 返回:
/// - 压缩结果文本
pub async fn compact_conversation_with_agent(
    config: &AppConfig,
    paths: &MiyuPaths,
    state: &StateStore,
    client: &OpenAiCompatibleClient,
    mode: AgentMode,
    keep_tail_turns: usize,
) -> Result<String> {
    let agent = Agent::new(
        config.clone(),
        paths,
        state.clone(),
        client.clone(),
        ToolRegistry::new(),
        mode,
    )?;
    format_compaction_result(agent.compact_conversation_now(keep_tail_turns).await?)
}

/// 从配置创建依赖并手动压缩当前会话。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `keep_tail_turns`: 保留的最近非运行轮次数量
///
/// 返回:
/// - 压缩结果文本
pub async fn compact_conversation_from_paths(
    paths: &MiyuPaths,
    keep_tail_turns: usize,
) -> Result<String> {
    AppConfig::init_files(paths)?;
    let config = AppConfig::load_or_default(paths)?;
    let state = StateStore::new(paths)?;
    state.init_files()?;
    let client = OpenAiCompatibleClient::from_config(&config, paths)?;
    compact_conversation_with_agent(
        &config,
        paths,
        &state,
        &client,
        AgentMode::Yolo,
        keep_tail_turns,
    )
    .await
}

/// 从配置创建依赖并压缩指定会话，不切换全局当前会话。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `session_id`: 会话 ID
/// - `keep_tail_turns`: 保留的最近非运行轮次数量
///
/// 返回:
/// - 压缩结果文本
pub async fn compact_session_from_paths(
    paths: &MiyuPaths,
    session_id: &str,
    keep_tail_turns: usize,
) -> Result<String> {
    AppConfig::init_files(paths)?;
    let config = AppConfig::load_or_default(paths)?;
    let state = StateStore::for_session(paths, session_id)?;
    state.init_files()?;
    let client = OpenAiCompatibleClient::from_config(&config, paths)?;
    compact_conversation_with_agent(
        &config,
        paths,
        &state,
        &client,
        AgentMode::Yolo,
        keep_tail_turns,
    )
    .await
}

/// 格式化压缩结果。
///
/// 参数:
/// - `turn_count`: 已压缩轮次数量
///
/// 返回:
/// - 压缩结果文本
fn format_compaction_result(turn_count: usize) -> Result<String> {
    Ok(if turn_count == 0 {
        t(
            "no old conversation turns are available for compaction",
            "没有可压缩的旧会话轮次",
        )
        .to_string()
    } else {
        format!(
            "{}: {turn_count}",
            t("compacted conversation turns", "已压缩会话轮次")
        )
    })
}
