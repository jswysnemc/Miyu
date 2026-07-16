import { describe, expect, it } from "vitest";
import {
  BUILTIN_SUBAGENT_PROFILES,
  createUniqueCustomSubagentProfile,
  mergeSubagentProfiles,
  removeCustomSubagentProfile,
  updateSubagentProfile
} from "./subagent-profile-state";

describe("子 Agent 档案状态", () => {
  it("合并内置档案时应用已存覆盖并保留自定义档案", () => {
    const profiles = mergeSubagentProfiles([
      { id: "general", name: "自定义通用名", thinking_level: "high", exposed: false },
      { id: "custom-review", name: "审查助手", description: "只做审查" }
    ]);

    expect(profiles.map((profile) => profile.id)).toEqual(["general", "explore", "custom-review"]);
    expect(profiles[0]).toMatchObject({ name: "自定义通用名", thinking_level: "high", exposed: false });
    expect(profiles[2]).toMatchObject({ id: "custom-review", name: "审查助手" });
  });

  it("没有存储值时返回新的内置档案列表", () => {
    const profiles = mergeSubagentProfiles();

    expect(profiles).toEqual(BUILTIN_SUBAGENT_PROFILES);
    expect(profiles).not.toBe(BUILTIN_SUBAGENT_PROFILES);
  });

  it("创建不与内置或已有自定义档案冲突的档案", () => {
    const created = createUniqueCustomSubagentProfile([
      ...BUILTIN_SUBAGENT_PROFILES,
      { id: "custom-1", name: "已有自定义" },
      { id: "custom-3", name: "另一个自定义" }
    ]);

    expect(created).toMatchObject({ id: "custom-2", name: "自定义子 Agent 2", exposed: true });
  });

  it("更新任意档案并保护其标识", () => {
    const profiles = [
      ...BUILTIN_SUBAGENT_PROFILES,
      { id: "custom-1", name: "旧名称", exposed: true }
    ];
    const updated = updateSubagentProfile(profiles, "custom-1", { id: "other", name: "新名称" });
    const builtinUpdated = updateSubagentProfile(updated, "general", { name: "项目通用 Agent" });

    expect(updated.find((profile) => profile.id === "custom-1")).toMatchObject({ id: "custom-1", name: "新名称" });
    expect(builtinUpdated.find((profile) => profile.id === "general")?.name).toBe("项目通用 Agent");
  });

  it("只删除自定义档案，内置档案始终保留", () => {
    const profiles = [...BUILTIN_SUBAGENT_PROFILES, { id: "custom-1", name: "自定义" }];

    expect(removeCustomSubagentProfile(profiles, "custom-1").map((profile) => profile.id)).toEqual(["general", "explore"]);
    expect(removeCustomSubagentProfile(profiles, "general")).toEqual(profiles);
  });
});
