export type AgentProfile = {
  id: string;
  name: string;
  system_prompt: string;
  enabled_tools: string[];
  skills_full: string[];
  skills_named: string[];
  provider_id: string;
  model: string;
  thinking_level: string;
};

export type AgentChoice = {
  id: string;
  name: string;
};
