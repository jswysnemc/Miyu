use serde::{Deserialize, Serialize};

pub const DEFAULT_AGENT_ID: &str = "default";

/// 仅在单轮运行期间生效的 Agent 能力覆盖。
#[derive(Debug, Clone, PartialEq)]
pub struct AgentRuntimeOverride {
    /// 允许使用的工具名称
    pub enabled_tools: Vec<String>,
    /// 完整暴露的 skills
    pub skills_full: Vec<String>,
    /// 仅暴露名称的 skills
    pub skills_named: Vec<String>,
}

/// Agent 配置档案。
///
/// 描述一个可复用的 agent 预设：系统提示词、启用的工具集合以及暴露的 skills。
/// 本期只提供配置存取与 Web 端编辑界面，运行时尚未消费这些字段。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentProfile {
    /// Agent 唯一标识
    pub id: String,
    /// Agent 显示名称
    pub name: String,
    /// 系统提示词全文
    #[serde(default)]
    pub system_prompt: String,
    /// 启用的工具，可填写工具名或工具分组名
    #[serde(default)]
    pub enabled_tools: Vec<String>,
    /// 完整启用的 skills：加载名称与描述
    #[serde(default)]
    pub skills_full: Vec<String>,
    /// 半启用的 skills：仅暴露名称
    #[serde(default)]
    pub skills_named: Vec<String>,
}

/// 子智能体运行配置。
///
/// 控制 task 工具启动的进程内子智能体使用哪个供应商与模型。
/// 留空时子智能体沿用主对话当前的供应商与模型。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SubagentConfig {
    /// 子智能体使用的供应商 id，空表示沿用主对话
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub provider_id: String,
    /// 子智能体使用的模型，空表示沿用该供应商默认模型
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
}
