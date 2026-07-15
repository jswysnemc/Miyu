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
    /// 可选供应商 id，空表示沿用当前供应商
    #[serde(default)]
    pub provider_id: String,
    /// 可选模型，空表示沿用供应商当前模型
    #[serde(default)]
    pub model: String,
    /// 可选思考等级，auto 表示沿用当前配置
    #[serde(default = "default_agent_thinking_level")]
    pub thinking_level: String,
}

/// 可由主 Agent 选择的子 Agent 档案。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubagentProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub provider_id: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_agent_thinking_level")]
    pub thinking_level: String,
    #[serde(default = "default_true")]
    pub exposed: bool,
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
    #[serde(default = "default_agent_thinking_level")]
    pub thinking_level: String,
    #[serde(default)]
    pub default_profile: String,
    #[serde(default)]
    pub profiles: Vec<SubagentProfile>,
}

impl SubagentConfig {
    /// 返回包含内置通用与探索档案的完整配置列表。
    pub fn resolved_profiles(&self) -> Vec<SubagentProfile> {
        [builtin_general_profile(), builtin_explore_profile()]
            .into_iter()
            .map(|builtin| {
                self.profiles
                    .iter()
                    .find(|profile| profile.id == builtin.id)
                    .cloned()
                    .unwrap_or(builtin)
            })
            .chain(
                self.profiles
                    .iter()
                    .filter(|profile| !matches!(profile.id.as_str(), "general" | "explore"))
                    .cloned(),
            )
            .collect()
    }

    /// 解析主 Agent 请求的已暴露子 Agent 档案。
    ///
    /// 参数:
    /// - `requested`: 主 Agent 显式选择的档案 id
    ///
    /// 返回:
    /// - 已暴露的子 Agent 档案
    pub fn resolve_profile(&self, requested: Option<&str>) -> Option<SubagentProfile> {
        let requested = requested
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or_else(|| {
                (!self.default_profile.trim().is_empty()).then_some(self.default_profile.trim())
            })
            .unwrap_or("general");
        self.resolved_profiles()
            .into_iter()
            .find(|profile| profile.exposed && profile.id == requested)
    }
}

fn builtin_general_profile() -> SubagentProfile {
    SubagentProfile {
        id: "general".to_string(),
        name: "通用 Agent".to_string(),
        description: "适合实现、测试、文档和常规工程任务".to_string(),
        system_prompt: String::new(),
        provider_id: String::new(),
        model: String::new(),
        thinking_level: "auto".to_string(),
        exposed: true,
    }
}

fn builtin_explore_profile() -> SubagentProfile {
    SubagentProfile {
        id: "explore".to_string(),
        name: "探索 Agent".to_string(),
        description: "适合只读检索、代码定位和资料探索".to_string(),
        system_prompt: String::new(),
        provider_id: String::new(),
        model: String::new(),
        thinking_level: "auto".to_string(),
        exposed: true,
    }
}

fn default_agent_thinking_level() -> String {
    "auto".to_string()
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证内置通用与探索子 Agent 可被配置覆盖。
    #[test]
    fn resolves_builtin_subagent_profile_overrides() {
        let config = SubagentConfig {
            profiles: vec![SubagentProfile {
                id: "explore".to_string(),
                name: "代码探索".to_string(),
                description: "定位代码".to_string(),
                system_prompt: "只读检索".to_string(),
                provider_id: "custom".to_string(),
                model: "model-x".to_string(),
                thinking_level: "high".to_string(),
                exposed: true,
            }],
            default_profile: "explore".to_string(),
            ..SubagentConfig::default()
        };
        let profile = config.resolve_profile(None).unwrap();
        assert_eq!(profile.id, "explore");
        assert_eq!(profile.thinking_level, "high");
    }

    /// 验证未暴露子 Agent 不可被主 Agent 选择。
    #[test]
    fn hides_unexposed_subagent_profiles() {
        let mut config = SubagentConfig::default();
        config.profiles.push(SubagentProfile {
            id: "explore".to_string(),
            name: "探索".to_string(),
            description: String::new(),
            system_prompt: String::new(),
            provider_id: String::new(),
            model: String::new(),
            thinking_level: "auto".to_string(),
            exposed: false,
        });
        assert!(config.resolve_profile(Some("explore")).is_none());
    }
}
