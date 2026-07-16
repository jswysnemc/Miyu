import { describe, expect, it } from "vitest";
import type { AgentOptions } from "./agents-types";
import {
  buildVisibleAgentProfiles,
  createUniqueAgentProfile,
  normalizeAgentProfile,
  removeAgentProfile,
  updateAgentProfile
} from "./agent-profile-state";

const options: AgentOptions = {
  tools: [{ name: "read_file", group: "文件" }, { name: "run_command", group: "命令" }],
  skills: [{ name: "review", description: "代码审查" }]
};

describe("主 Agent 档案状态", () => {
  it("为可选字段提供稳定的空值和默认思考等级", () => {
    expect(normalizeAgentProfile({ id: "review", name: "" })).toEqual({
      id: "review",
      name: "review",
      system_prompt: "",
      enabled_tools: [],
      skills_full: [],
      skills_named: [],
      provider_id: "",
      model: "",
      thinking_level: "auto"
    });
  });

  it("补充虚拟默认档案且不写回原数组", () => {
    const stored = [{ id: "review", name: "审查", enabled_tools: ["read_file"] }];
    const visible = buildVisibleAgentProfiles(stored, options);

    expect(visible.map((profile) => profile.id)).toEqual(["default", "review"]);
    expect(visible[0].enabled_tools).toEqual(["read_file", "run_command"]);
    expect(stored).toEqual([{ id: "review", name: "审查", enabled_tools: ["read_file"] }]);
  });

  it("已存默认档案优先于虚拟默认档案", () => {
    const visible = buildVisibleAgentProfiles([
      { id: "default", name: "项目默认", thinking_level: "high" }
    ], options);

    expect(visible).toHaveLength(1);
    expect(visible[0]).toMatchObject({ id: "default", name: "项目默认", thinking_level: "high" });
  });

  it("创建不冲突的自定义 Agent", () => {
    const created = createUniqueAgentProfile([
      { id: "agent-1", name: "一个" },
      { id: "agent-3", name: "三个" }
    ], options);

    expect(created).toMatchObject({ id: "agent-2", name: "新 Agent 2" });
    expect(created.enabled_tools).toEqual(["read_file", "run_command"]);
    expect(created.skills_full).toEqual(["review"]);
  });

  it("按标识更新和删除档案，不改变原数组", () => {
    const profiles = [{ id: "review", name: "审查" }, { id: "writer", name: "写作" }];
    const updated = updateAgentProfile(profiles, "review", { name: "代码审查" });
    const removed = removeAgentProfile(updated, "writer");

    expect(updated).toEqual([{ id: "review", name: "代码审查" }, { id: "writer", name: "写作" }]);
    expect(removed).toEqual([{ id: "review", name: "代码审查" }]);
    expect(profiles[0].name).toBe("审查");
  });
});
