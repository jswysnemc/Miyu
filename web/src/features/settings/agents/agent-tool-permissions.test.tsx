import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { AgentToolPermissions, updateEnabledTools } from "./agent-tool-permissions";

const tools = [
  { name: "read_file", group: "文件" },
  { name: "edit_file", group: "文件" },
  { name: "run_command", group: "命令" }
];

describe("AgentToolPermissions", () => {
  it("渲染搜索、全局批量操作和分组选择", () => {
    const html = renderToStaticMarkup(
      <AgentToolPermissions tools={tools} enabled={["read_file"]} onChange={vi.fn()} />
    );

    expect(html).toContain('placeholder="搜索工具或分组"');
    expect(html).toContain("全部启用");
    expect(html).toContain("全部清空");
    expect(html).toContain('aria-label="选择文件分组的全部工具"');
    expect(html).toContain("1/2");
  });

  it("更新单个工具时保留其他已启用工具且不产生重复项", () => {
    expect(updateEnabledTools(["read_file", "run_command"], ["edit_file"], true))
      .toEqual(["read_file", "run_command", "edit_file"]);
    expect(updateEnabledTools(["read_file", "edit_file"], ["read_file"], false))
      .toEqual(["edit_file"]);
  });
});
