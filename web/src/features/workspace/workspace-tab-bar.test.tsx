import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { WorkspaceTabBar } from "./workspace-tab-bar";

describe("WorkspaceTabBar", () => {
  it("保留可关闭标签的键盘可访问按钮", () => {
    const html = renderToStaticMarkup(
      <WorkspaceTabBar
        tabs={[
          { id: "file:README.md", type: "files", title: "README.md", path: "README.md", closable: true },
          { id: "diff", type: "diff", title: "Git", closable: false }
        ]}
        activeTabId="file:README.md"
        maximized={false}
        onActivate={vi.fn()}
        onClose={vi.fn()}
        onAdd={vi.fn()}
        onToggleMaximized={vi.fn()}
        onCollapse={vi.fn()}
      />
    );

    expect(html).toContain('class="workspace-tab-close"');
    expect(html).toContain('aria-label="关闭 README.md"');
    expect(html.match(/workspace-tab-close/g)).toHaveLength(1);
  });
});
