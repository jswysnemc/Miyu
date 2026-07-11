import { describe, expect, it } from "vitest";
import { calculateAnchoredPopoverPosition } from "./anchored-popover-position";

describe("anchored popover position", () => {
  it("在移动端将左对齐菜单限制在视口内部", () => {
    expect(calculateAnchoredPopoverPosition(
      { left: 28, right: 142, bottom: 48 },
      { viewportWidth: 375, preferredWidth: 520, minimumWidth: 240, align: "left" }
    )).toEqual({ left: 12, top: 54, width: 351 });
  });

  it("将右对齐菜单贴近触发器并限制左边界", () => {
    expect(calculateAnchoredPopoverPosition(
      { left: 195, right: 286, bottom: 48 },
      { viewportWidth: 375, preferredWidth: 220, minimumWidth: 180, align: "right" }
    )).toEqual({ left: 66, top: 54, width: 220 });
  });
});
