export type AgentProfile = {
  id: string;
  name: string;
  system_prompt: string;
  enabled_tools: string[];
  skills_full: string[];
  skills_named: string[];
};

export type AgentChoice = {
  id: string;
  name: string;
};
