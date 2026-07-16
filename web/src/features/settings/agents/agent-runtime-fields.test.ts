import { describe, expect, it } from "vitest";
import type { AppConfig } from "../../../api/contracts";
import { buildAgentModelNames } from "./agent-runtime-fields";

const config = {
  active_provider: "active",
  providers: [
    { id: "active", display_name: "当前供应商", models: ["active-model"] },
    { id: "child", display_name: "子供应商", models: ["child-model"] }
  ]
} as AppConfig;

describe("buildAgentModelNames", () => {
  it("供应商留空时使用继承供应商的模型列表", () => {
    expect(buildAgentModelNames(config, "", "child", "")).toEqual(["child-model"]);
  });

  it("保留模型列表外的历史覆盖值", () => {
    expect(buildAgentModelNames(config, "", undefined, "legacy-model")).toEqual(["legacy-model", "active-model"]);
  });
});
