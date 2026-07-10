/** Agent 配置档案，对应后端 AppConfig.agents 数组元素。 */
export type AgentProfile = {
  id: string;
  name: string;
  system_prompt: string;
  enabled_tools: string[];
  skills_full: string[];
  skills_named: string[];
};

/** 内置工具选项，含用途分组。 */
export type AgentToolOption = {
  name: string;
  group: string;
};

/** Skill 选项，含名称与描述。 */
export type AgentSkillOption = {
  name: string;
  description: string;
};

/** GET /api/agent-options 响应体。 */
export type AgentOptions = {
  tools: AgentToolOption[];
  skills: AgentSkillOption[];
};
