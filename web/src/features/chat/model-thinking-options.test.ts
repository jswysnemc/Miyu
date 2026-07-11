import { describe, expect, it } from "vitest";
import { THINKING_OPTIONS, thinkingLevelLabel } from "./model-thinking-options";

describe("model thinking options", () => {
  it("为全部推理等级提供唯一选项", () => {
    expect(new Set(THINKING_OPTIONS.map((option) => option.value)).size).toBe(7);
  });

  it("返回紧凑中文展示名称", () => {
    expect(thinkingLevelLabel("xhigh")).toBe("极高");
    expect(thinkingLevelLabel("none")).toBe("关闭");
  });
});
