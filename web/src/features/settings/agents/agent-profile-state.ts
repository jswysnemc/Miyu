import type { AgentProfileConfig } from "../../../api/contracts";
import { buildDefaultAgent, DEFAULT_AGENT_ID } from "../../agents/agent-options";
import type { AgentProfile } from "../../agents/agent-types";
import type { AgentOptions } from "./agents-types";

/**
 * 将配置文件中的主 Agent 档案转换成编辑器可直接使用的完整档案。
 *
 * @param profile 配置文件中的主 Agent 档案
 * @param defaults 可选的字段默认值
 * @returns 所有编辑字段均存在的主 Agent 档案
 */
export function normalizeAgentProfile(
  profile: AgentProfileConfig,
  defaults: Partial<AgentProfile> = {}
): AgentProfile {
  const id = profile.id || defaults.id || "agent";
  return {
    id,
    name: profile.name || defaults.name || id,
    system_prompt: profile.system_prompt ?? defaults.system_prompt ?? "",
    enabled_tools: copyList(profile.enabled_tools ?? defaults.enabled_tools),
    skills_full: copyList(profile.skills_full ?? defaults.skills_full),
    skills_named: copyList(profile.skills_named ?? defaults.skills_named),
    provider_id: profile.provider_id ?? defaults.provider_id ?? "",
    model: profile.model ?? defaults.model ?? "",
    thinking_level: profile.thinking_level ?? defaults.thinking_level ?? "auto"
  };
}

/**
 * 构造设置界面可见的主 Agent 列表，缺少持久化默认档案时补充虚拟默认项。
 *
 * @param profiles 已存储的主 Agent 档案
 * @param options 当前可用的工具与 Skill 选项
 * @returns 可供界面展示的完整主 Agent 档案列表
 */
export function buildVisibleAgentProfiles(
  profiles: readonly AgentProfileConfig[] | undefined,
  options: AgentOptions
): AgentProfile[] {
  const normalized = (profiles ?? []).map((profile) => normalizeAgentProfile(profile));
  if (normalized.some((profile) => profile.id === DEFAULT_AGENT_ID)) return normalized;
  return [buildDefaultAgent(options), ...normalized];
}

/**
 * 创建一个启用全部当前选项的唯一自定义主 Agent 档案。
 *
 * @param profiles 已存储的主 Agent 档案
 * @param options 当前可用的工具与 Skill 选项
 * @param overrides 新档案需要覆盖的字段
 * @returns 尚未写入配置的全新主 Agent 档案
 */
export function createUniqueAgentProfile(
  profiles: readonly AgentProfileConfig[],
  options: AgentOptions,
  overrides: Partial<AgentProfile> = {}
): AgentProfile {
  const id = nextNumericId(profiles.map((profile) => profile.id), "agent");
  const number = id.slice("agent-".length);
  const { id: _ignoredId, ...fields } = overrides;
  return normalizeAgentProfile({
    id,
    name: `新 Agent ${number}`,
    system_prompt: "",
    enabled_tools: options.tools.map((tool) => tool.name),
    skills_full: options.skills.map((skill) => skill.name),
    skills_named: [],
    provider_id: "",
    model: "",
    thinking_level: "auto",
    ...fields
  });
}

/**
 * 更新指定主 Agent 档案，并保持档案标识不可变。
 *
 * @param profiles 已存储的主 Agent 档案
 * @param id 待更新档案的标识
 * @param patch 需要覆盖的可选字段
 * @returns 更新后的新数组
 */
export function updateAgentProfile(
  profiles: readonly AgentProfileConfig[],
  id: string,
  patch: Partial<AgentProfileConfig>
): AgentProfileConfig[] {
  const { id: _ignoredId, ...fields } = patch;
  return profiles.map((profile) => profile.id === id ? { ...profile, ...fields, id } : { ...profile });
}

/**
 * 删除指定主 Agent 档案。
 *
 * @param profiles 已存储的主 Agent 档案
 * @param id 待删除档案的标识
 * @returns 删除后的新数组
 */
export function removeAgentProfile(
  profiles: readonly AgentProfileConfig[],
  id: string
): AgentProfileConfig[] {
  return profiles.filter((profile) => profile.id !== id).map((profile) => ({ ...profile }));
}

/**
 * 复制可选字符串数组，避免状态函数修改调用方持有的数组。
 *
 * @param values 待复制的可选字符串数组
 * @returns 独立的字符串数组
 */
function copyList(values: readonly string[] | undefined): string[] {
  return Array.isArray(values) ? [...values] : [];
}

/**
 * 根据前缀找到第一个未使用的数字标识。
 *
 * @param ids 已使用的标识
 * @param prefix 新标识的前缀
 * @returns 第一个未占用的数字标识
 */
function nextNumericId(ids: readonly string[], prefix: string): string {
  const used = new Set(ids);
  let suffix = 1;
  while (used.has(`${prefix}-${suffix}`)) suffix += 1;
  return `${prefix}-${suffix}`;
}
