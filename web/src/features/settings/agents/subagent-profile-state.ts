import type { SubagentProfileConfig } from "../../../api/contracts";

/** 内置子 Agent 档案，始终显示在自定义档案之前。 */
export const BUILTIN_SUBAGENT_PROFILES: SubagentProfileConfig[] = [
  {
    id: "general",
    name: "通用 Agent",
    description: "适合实现、测试、文档和常规工程任务",
    thinking_level: "auto",
    exposed: true
  },
  {
    id: "explore",
    name: "探索 Agent",
    description: "适合只读检索、代码定位和资料探索",
    thinking_level: "auto",
    exposed: true
  }
];

/**
 * 合并内置与已存储的子 Agent 档案，并保留所有未占用内置标识的自定义档案。
 *
 * @param stored 已存储的子 Agent 档案
 * @returns 应用于设置界面的完整档案列表
 */
export function mergeSubagentProfiles(
  stored: readonly SubagentProfileConfig[] | undefined = undefined
): SubagentProfileConfig[] {
  const byId = new Map((stored ?? []).map((profile) => [profile.id, profile]));
  const builtinIds = new Set(BUILTIN_SUBAGENT_PROFILES.map((profile) => profile.id));
  const builtins = BUILTIN_SUBAGENT_PROFILES.map((profile) => ({
    ...profile,
    ...(byId.get(profile.id) ?? {})
  }));
  const custom = (stored ?? [])
    .filter((profile) => !builtinIds.has(profile.id))
    .map((profile) => ({ ...profile }));
  return [...builtins, ...custom];
}

/**
 * 创建一个不与内置或已有自定义档案冲突的新子 Agent。
 *
 * @param profiles 当前可见的子 Agent 档案
 * @param overrides 新档案需要覆盖的字段
 * @returns 尚未写入配置的自定义子 Agent 档案
 */
export function createUniqueCustomSubagentProfile(
  profiles: readonly SubagentProfileConfig[],
  overrides: Partial<SubagentProfileConfig> = {}
): SubagentProfileConfig {
  const id = nextCustomId(profiles.map((profile) => profile.id));
  const number = id.slice("custom-".length);
  const { id: _ignoredId, ...fields } = overrides;
  return {
    id,
    name: `自定义子 Agent ${number}`,
    description: "",
    system_prompt: "",
    provider_id: "",
    model: "",
    thinking_level: "auto",
    exposed: true,
    ...fields
  };
}

/**
 * 更新指定子 Agent，档案标识不会被修改。
 *
 * @param profiles 当前子 Agent 档案
 * @param id 待更新档案的标识
 * @param patch 需要覆盖的字段
 * @returns 更新后的新数组
 */
export function updateSubagentProfile(
  profiles: readonly SubagentProfileConfig[],
  id: string,
  patch: Partial<SubagentProfileConfig>
): SubagentProfileConfig[] {
  const { id: _ignoredId, ...fields } = patch;
  return profiles.map((profile) => profile.id === id ? { ...profile, ...fields, id } : { ...profile });
}

/**
 * 删除自定义子 Agent，内置档案不会被删除。
 *
 * @param profiles 当前子 Agent 档案
 * @param id 待删除档案的标识
 * @returns 删除后的新数组
 */
export function removeCustomSubagentProfile(
  profiles: readonly SubagentProfileConfig[],
  id: string
): SubagentProfileConfig[] {
  if (isBuiltinSubagentId(id)) return profiles.map((profile) => ({ ...profile }));
  return profiles.filter((profile) => profile.id !== id).map((profile) => ({ ...profile }));
}

/**
 * 判断标识是否属于受保护的内置子 Agent。
 *
 * @param id 待判断的子 Agent 标识
 * @returns 是否为内置子 Agent
 */
function isBuiltinSubagentId(id: string): boolean {
  return BUILTIN_SUBAGENT_PROFILES.some((profile) => profile.id === id);
}

/**
 * 根据 custom 前缀找到第一个未使用的数字标识。
 *
 * @param ids 已使用的子 Agent 标识
 * @returns 第一个未占用的自定义标识
 */
function nextCustomId(ids: readonly string[]): string {
  const used = new Set(ids);
  let suffix = 1;
  while (used.has(`custom-${suffix}`)) suffix += 1;
  return `custom-${suffix}`;
}
