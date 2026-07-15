import { useEffect, useState } from "react";
import { DiffPane } from "./diff-pane";
import { EditorPane } from "./editor-pane";
import { FileTree } from "./file-tree";
import { TerminalDock } from "../terminal/terminal-dock";
import { SubagentWorkspace } from "../subagents/subagent-workspace";
import type { TerminalManager } from "../terminal/use-terminal-manager";
import { createWorkspacePanelTab, type PaneTab, type WorkspacePanelTab } from "./workspace-tab";
import { WorkspaceTabBar } from "./workspace-tab-bar";
import "./workspace-pane.css";

type WorkspacePaneProps = {
  selectedFile: string | null;
  activeType: PaneTab;
  onActiveTypeChange: (tab: PaneTab) => void;
  onSelectFile: (path: string) => void;
  onClearFile: () => void;
  terminalManager: TerminalManager;
};

/**
 * 渲染带 Cursor 风格顶部标签栏的右侧工作区。
 *
 * @param props 文件选择、活动类型与终端状态
 * @returns 工作区面板
 */
export function WorkspacePane({
  selectedFile,
  activeType,
  onActiveTypeChange,
  onSelectFile,
  onClearFile,
  terminalManager
}: WorkspacePaneProps) {
  const [fileTreeOpen, setFileTreeOpen] = useState(false);
  const [tabs, setTabs] = useState<WorkspacePanelTab[]>(() => [
    createWorkspacePanelTab("files", { title: "编辑器", closable: false })
  ]);
  const [activeTabId, setActiveTabId] = useState(tabs[0]?.id ?? null);

  // 打开文件时：复用同路径标签 / 填充空编辑器 / 新建文件标签。
  useEffect(() => {
    if (!selectedFile) return;
    setTabs((current) => {
      const existing = current.find((tab) => tab.type === "files" && tab.path === selectedFile);
      if (existing) {
        setActiveTabId(existing.id);
        return current;
      }
      const emptyEditor = current.find((tab) => tab.type === "files" && !tab.path);
      if (emptyEditor) {
        setActiveTabId(emptyEditor.id);
        return current.map((tab) =>
          tab.id === emptyEditor.id
            ? {
                ...tab,
                path: selectedFile,
                title: selectedFile.split("/").filter(Boolean).at(-1) ?? selectedFile,
                closable: true
              }
            : tab
        );
      }
      const created = createWorkspacePanelTab("files", { path: selectedFile });
      setActiveTabId(created.id);
      return [...current, created];
    });
    onActiveTypeChange("files");
  }, [onActiveTypeChange, selectedFile]);

  // 外部活动栏切换类型时，激活已有同类型标签或补一个。
  useEffect(() => {
    setTabs((current) => {
      const existing = current.find((tab) => tab.type === activeType);
      if (existing) {
        setActiveTabId((id) => (id === existing.id ? id : existing.id));
        return current;
      }
      const created = createWorkspacePanelTab(activeType, {
        closable: true
      });
      setActiveTabId(created.id);
      return [...current, created];
    });
  }, [activeType]);

  const activeTab = tabs.find((tab) => tab.id === activeTabId) ?? tabs[0] ?? null;

  /**
   * 添加指定类型的工作区标签。
   *
   * @param type 面板类型
   */
  const addTab = (type: PaneTab) => {
    if (type === "files") {
      const created = createWorkspacePanelTab("files", { title: "编辑器" });
      setTabs((current) => [...current, created]);
      setActiveTabId(created.id);
      onActiveTypeChange("files");
      onClearFile();
      return;
    }
    const existing = tabs.find((tab) => tab.type === type);
    if (existing) {
      setActiveTabId(existing.id);
      onActiveTypeChange(type);
      return;
    }
    const created = createWorkspacePanelTab(type);
    setTabs((current) => [...current, created]);
    setActiveTabId(created.id);
    onActiveTypeChange(type);
  };

  /**
   * 关闭指定标签。
   *
   * @param id 标签 ID
   */
  const closeTab = (id: string) => {
    setTabs((current) => {
      if (current.length <= 1) return current;
      const index = current.findIndex((tab) => tab.id === id);
      if (index < 0) return current;
      const closing = current[index];
      const next = current.filter((tab) => tab.id !== id);
      if (activeTabId === id) {
        const fallback = next[Math.max(0, index - 1)] ?? next[0];
        setActiveTabId(fallback?.id ?? null);
        if (fallback) onActiveTypeChange(fallback.type);
      }
      if (closing?.type === "files" && closing.path && closing.path === selectedFile) {
        const remainingFile = next.find((tab) => tab.type === "files" && tab.path);
        if (remainingFile?.path) onSelectFile(remainingFile.path);
        else onClearFile();
      }
      return next;
    });
  };

  return (
    <div className="workspace-pane">
      <WorkspaceTabBar
        tabs={tabs}
        activeTabId={activeTab?.id ?? null}
        onActivate={(id) => {
          setActiveTabId(id);
          const tab = tabs.find((item) => item.id === id);
          if (!tab) return;
          onActiveTypeChange(tab.type);
          if (tab.type === "files" && tab.path) onSelectFile(tab.path);
        }}
        onClose={closeTab}
        onAdd={addTab}
      />
      <div className="pane-body">
        {activeTab?.type === "files" && (
          <div className={fileTreeOpen ? "files-layout file-tree-open" : "files-layout file-tree-closed"}>
            <EditorPane
              path={activeTab.path ?? selectedFile}
              onSelectFile={onSelectFile}
              fileTreeOpen={fileTreeOpen}
              onToggleFileTree={() => setFileTreeOpen((value) => !value)}
            />
            {fileTreeOpen && (
              <FileTree
                selectedFile={activeTab.path ?? selectedFile}
                onSelectFile={onSelectFile}
                onClearFile={onClearFile}
                onClose={() => setFileTreeOpen(false)}
              />
            )}
          </div>
        )}
        {activeTab?.type === "diff" && <DiffPane />}
        {activeTab?.type === "terminal" && (
          <TerminalDock manager={terminalManager} onClose={() => closeTab(activeTab.id)} />
        )}
        {activeTab?.type === "subagents" && <SubagentWorkspace />}
      </div>
    </div>
  );
}
