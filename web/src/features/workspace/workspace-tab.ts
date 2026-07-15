export type PaneTab = "files" | "diff" | "terminal" | "tasks" | "subagents";

export type WorkspacePanelTab = {
  id: string;
  type: PaneTab;
  title: string;
  path?: string;
  closable: boolean;
};

/**
 * 创建工作区面板标签。
 *
 * @param type 面板类型
 * @param options 标题与文件路径
 * @returns 工作区标签
 */
export function createWorkspacePanelTab(
  type: PaneTab,
  options: { title?: string; path?: string; closable?: boolean } = {}
): WorkspacePanelTab {
  const path = options.path;
  if (type === "files") {
    const title = options.title ?? (path ? path.split("/").filter(Boolean).at(-1) ?? path : "编辑器");
    return {
      id: path ? `file:${path}` : `files:${crypto.randomUUID()}`,
      type,
      title,
      path,
      closable: options.closable ?? true
    };
  }
  const defaults: Record<Exclude<PaneTab, "files">, string> = {
    diff: "Git",
    terminal: "终端",
    tasks: "后台任务",
    subagents: "子智能体"
  };
  return {
    id: `${type}:${crypto.randomUUID()}`,
    type,
    title: options.title ?? defaults[type],
    closable: options.closable ?? true
  };
}

/**
 * 返回指定类型面板的默认标题。
 *
 * @param type 面板类型
 * @returns 标题
 */
export function paneTabLabel(type: PaneTab): string {
  return {
    files: "编辑器",
    diff: "Git",
    terminal: "终端",
    tasks: "后台任务",
    subagents: "子智能体"
  }[type];
}
