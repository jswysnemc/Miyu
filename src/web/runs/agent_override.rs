use crate::config::{AgentRuntimeOverride, AppConfig, DEFAULT_AGENT_ID};
use anyhow::{bail, Result};

/// 把指定 Agent 档案应用到单轮内存配置。
///
/// 参数:
/// - `config`: 当前应用配置
/// - `agent_id`: 主界面选择的 Agent 标识
///
/// 返回:
/// - 已应用系统提示词、工具和 skills 策略的配置
pub(super) fn apply_agent_override(
    mut config: AppConfig,
    agent_id: Option<&str>,
) -> Result<AppConfig> {
    // 1. 显式 agent_id 优先,缺省时回退到全局默认 agent 配置
    let explicit = agent_id.map(str::trim).filter(|value| !value.is_empty());
    let fallback = config
        .default_agent
        .clone()
        .filter(|value| !value.trim().is_empty());
    let Some(agent_id) = explicit
        .map(str::to_string)
        .or(fallback)
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(config);
    };
    let agent_id = agent_id.trim();
    let profile = config
        .agents
        .iter()
        .find(|agent| agent.id == agent_id)
        .cloned();
    let Some(profile) = profile else {
        if agent_id == DEFAULT_AGENT_ID {
            return Ok(config);
        }
        bail!("agent not found: {agent_id}");
    };
    // 1. 非空提示词替换当前基础系统提示词，空值继续继承全局默认
    if !profile.system_prompt.trim().is_empty() {
        config.system_prompt_file = None;
        config.system_prompt = Some(profile.system_prompt);
    }
    // 2. Agent 可独立覆盖供应商、模型和思考等级
    if !profile.provider_id.trim().is_empty() {
        config.active_provider = profile.provider_id.clone();
    }
    if let Some(provider) = config
        .providers
        .iter_mut()
        .find(|provider| provider.id == config.active_provider)
    {
        if !profile.model.trim().is_empty() {
            provider.default_model = profile.model.clone();
        }
        if !profile.thinking_level.trim().is_empty() && profile.thinking_level != "auto" {
            provider.thinking_level = profile.thinking_level.clone();
        }
    }
    // 3. 工具和 skills 策略仅用于本轮注册表与提示词组装
    config.agent_runtime = Some(AgentRuntimeOverride {
        enabled_tools: profile.enabled_tools,
        skills_full: profile.skills_full,
        skills_named: profile.skills_named,
    });
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentProfile;

    #[test]
    fn keeps_existing_runtime_for_virtual_default_agent() {
        let config = AppConfig::default();
        let resolved = apply_agent_override(config, Some(DEFAULT_AGENT_ID)).unwrap();
        assert!(resolved.agent_runtime.is_none());
        assert!(resolved.system_prompt_file.is_some());
    }

    #[test]
    fn applies_configured_agent_capabilities() {
        let mut config = AppConfig::default();
        config.agents.push(AgentProfile {
            id: "reviewer".to_string(),
            name: "审查".to_string(),
            system_prompt: "只审查代码".to_string(),
            enabled_tools: vec!["read_file".to_string()],
            skills_full: vec!["code-review".to_string()],
            skills_named: vec!["research".to_string()],
            ..AgentProfile::default()
        });
        let resolved = apply_agent_override(config, Some("reviewer")).unwrap();
        assert_eq!(resolved.system_prompt.as_deref(), Some("只审查代码"));
        let runtime = resolved.agent_runtime.unwrap();
        assert_eq!(runtime.enabled_tools, vec!["read_file"]);
        assert_eq!(runtime.skills_full, vec!["code-review"]);
        assert_eq!(runtime.skills_named, vec!["research"]);
    }

    #[test]
    fn rejects_unknown_agent() {
        let error = apply_agent_override(AppConfig::default(), Some("missing")).unwrap_err();
        assert!(error.to_string().contains("agent not found"));
    }

    #[test]
    fn falls_back_to_default_agent_when_id_absent() {
        let mut config = AppConfig::default();
        config.agents.push(AgentProfile {
            id: "writer".to_string(),
            name: "写作".to_string(),
            system_prompt: "专注写作".to_string(),
            enabled_tools: vec!["read_file".to_string()],
            skills_full: Vec::new(),
            skills_named: Vec::new(),
            ..AgentProfile::default()
        });
        config.default_agent = Some("writer".to_string());
        let resolved = apply_agent_override(config, None).unwrap();
        assert_eq!(resolved.system_prompt.as_deref(), Some("专注写作"));
    }

    #[test]
    fn explicit_agent_overrides_default_agent() {
        let mut config = AppConfig::default();
        config.agents.push(AgentProfile {
            id: "writer".to_string(),
            name: "写作".to_string(),
            system_prompt: "专注写作".to_string(),
            enabled_tools: Vec::new(),
            skills_full: Vec::new(),
            skills_named: Vec::new(),
            ..AgentProfile::default()
        });
        config.default_agent = Some("writer".to_string());
        // 显式传入虚拟默认 agent 时,应忽略 default_agent 回退
        let resolved = apply_agent_override(config, Some(DEFAULT_AGENT_ID)).unwrap();
        assert!(resolved.agent_runtime.is_none());
    }
}
