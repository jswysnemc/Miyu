import { describe, expect, it } from "vitest";
import { clampWorkspaceWidth, parseWorkspaceLayout } from "./workspace-layout-state";

describe("workspace layout state", () => {
  it("clamps the panel width to the supported range", () => {
    expect(clampWorkspaceWidth(120, 1440)).toBe(320);
    expect(clampWorkspaceWidth(900, 1440)).toBe(830);
    expect(clampWorkspaceWidth(700, 1080)).toBe(470);
  });

  it("restores persisted visibility and width", () => {
    expect(parseWorkspaceLayout('{"workspaceOpen":false,"workspaceWidth":480}', 1440)).toEqual({
      chatOpen: true,
      workspaceOpen: false,
      workspaceWidth: 480,
      workspaceMaximized: false,
      terminalOpen: false,
      terminalHeight: 280,
      swapped: false
    });
  });

  it("uses defaults for invalid persisted content", () => {
    expect(parseWorkspaceLayout("invalid", 1440)).toEqual({
      chatOpen: true,
      workspaceOpen: true,
      workspaceWidth: 520,
      workspaceMaximized: false,
      terminalOpen: false,
      terminalHeight: 280,
      swapped: false
    });
  });
});
