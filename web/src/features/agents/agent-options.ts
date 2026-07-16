import type { AppConfig } from "../../api/contracts";
import type { AgentOptions } from "../settings/agents/agents-types";
import type { AgentChoice, AgentProfile } from "./agent-types";

export const DEFAULT_AGENT_ID = "default";

/** 从应用配置中读取 Agent 档案。 */
export function readAgentProfiles(config: AppConfig): AgentProfile[] {
  const value = (config as { agents?: AgentProfile[] }).agents;
  return Array.isArray(value) ? value : [];
}

/** 构造继承当前运行行为的默认 Agent 档案。 */
export function buildDefaultAgent(options: AgentOptions): AgentProfile {
  return {
    id: DEFAULT_AGENT_ID,
    name: "默认 Agent",
    system_prompt: "",
    enabled_tools: options.tools.map((tool) => tool.name),
    skills_full: options.skills.map((skill) => skill.name),
    skills_named: [],
    provider_id: "",
    model: "",
    thinking_level: "auto"
  };
}

/** 构造主界面可选择的 Agent，配置缺失时补充虚拟默认项。 */
export function buildAgentChoices(config: AppConfig): AgentChoice[] {
  const profiles = readAgentProfiles(config);
  const choices = profiles.map(({ id, name }) => ({ id, name: name || id }));
  if (!choices.some((choice) => choice.id === DEFAULT_AGENT_ID)) {
    choices.unshift({ id: DEFAULT_AGENT_ID, name: "默认 Agent" });
  }
  return choices;
}

/** 从本地偏好和有效选项中解析当前 Agent。 */
export function resolveAgentChoice(choices: AgentChoice[], preferredId: string | null): AgentChoice | null {
  return choices.find((choice) => choice.id === preferredId)
    ?? choices.find((choice) => choice.id === DEFAULT_AGENT_ID)
    ?? choices[0]
    ?? null;
}
