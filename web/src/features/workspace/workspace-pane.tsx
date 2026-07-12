import { useState } from "react";
import { DiffPane } from "./diff-pane";
import { EditorPane } from "./editor-pane";
import { FileTree } from "./file-tree";
import { TerminalDock } from "../terminal/terminal-dock";
import { SubagentWorkspace } from "../subagents/subagent-workspace";
import type { TerminalManager } from "../terminal/use-terminal-manager";
import type { PaneTab } from "./workspace-tab";
import "./workspace-pane.css";

type WorkspacePaneProps = {
  selectedFile: string | null;
  tab: PaneTab;
  onTabChange: (tab: PaneTab) => void;
  onSelectFile: (path: string) => void;
  onClearFile: () => void;
  terminalManager: TerminalManager;
};

/**
 * 渲染文件、变更、终端和子智能体工作区内容。
 *
 * 视图切换与布局控制由悬浮活动栏负责,本组件只承载当前视图内容。
 *
 * @param props 文件选择状态与当前视图
 * @returns 右侧工作区面板
 */
export function WorkspacePane({ selectedFile, tab, onTabChange, onSelectFile, onClearFile, terminalManager }: WorkspacePaneProps) {
  const [fileTreeOpen, setFileTreeOpen] = useState(false);
  return (
    <div className="workspace-pane">
      <div className="pane-body">
        {tab === "files" && (
          <div className={fileTreeOpen ? "files-layout file-tree-open" : "files-layout file-tree-closed"}>
            <EditorPane path={selectedFile} onSelectFile={onSelectFile} fileTreeOpen={fileTreeOpen} onToggleFileTree={() => setFileTreeOpen((value) => !value)} />
            {fileTreeOpen && <FileTree selectedFile={selectedFile} onSelectFile={onSelectFile} onClearFile={onClearFile} onClose={() => setFileTreeOpen(false)} />}
          </div>
        )}
        {tab === "diff" && <DiffPane />}
        {tab === "terminal" && <TerminalDock manager={terminalManager} onClose={() => onTabChange("files")} />}
        {tab === "subagents" && <SubagentWorkspace />}
      </div>
    </div>
  );
}
