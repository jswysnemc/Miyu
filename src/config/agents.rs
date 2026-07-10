use serde::{Deserialize, Serialize};

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
