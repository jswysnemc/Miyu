import { ArrowLeftRight, FileCode2, GitCompareArrows, Maximize2, Minimize2, PanelBottomClose, PanelBottomOpen, PanelLeftClose, PanelRightClose, SquareTerminal } from "lucide-react";
import { useState } from "react";
import { DiffPane } from "./diff-pane";
import { EditorPane } from "./editor-pane";
import { FileTree } from "./file-tree";
import { TerminalDock } from "../terminal/terminal-dock";
import type { TerminalManager } from "../terminal/use-terminal-manager";
import "./workspace-pane.css";

type PaneTab = "files" | "diff" | "terminal";

type WorkspacePaneProps = {
  selectedFile: string | null;
  onSelectFile: (path: string) => void;
  onClearFile: () => void;
  onClose: () => void;
  onToggleChat: () => void;
  onToggleMaximized: () => void;
  onToggleTerminal: () => void;
  onToggleSwapped: () => void;
  terminalOpen: boolean;
  maximized: boolean;
  terminalManager: TerminalManager;
};

/**
 * 渲染文件、变更和终端工作区。
 *
 * @param props 文件选择状态和面板控制回调
 * @returns 右侧工作区面板
 */
export function WorkspacePane({ selectedFile, onSelectFile, onClearFile, onClose, onToggleChat, onToggleMaximized, onToggleTerminal, onToggleSwapped, terminalOpen, maximized, terminalManager }: WorkspacePaneProps) {
  const [tab, setTab] = useState<PaneTab>("files");
  const [fileTreeOpen, setFileTreeOpen] = useState(false);
  return (
    <div className="workspace-pane">
      <div className="pane-body">
        {tab === "files" && (
          <div className={fileTreeOpen ? "files-layout file-tree-open" : "files-layout file-tree-closed"}>
            <EditorPane path={selectedFile} onSelectFile={onSelectFile} fileTreeOpen={fileTreeOpen} onToggleFileTree={() => setFileTreeOpen((value) => !value)} />
            {fileTreeOpen && <FileTree selectedFile={selectedFile} onSelectFile={onSelectFile} onClearFile={onClearFile} />}
          </div>
        )}
        {tab === "diff" && <DiffPane />}
        {tab === "terminal" && <TerminalDock manager={terminalManager} onClose={() => setTab("files")} />}
      </div>
      <nav className="workspace-activity-rail" aria-label="侧边工作台">
        <ActivityButton active={tab === "files"} onClick={() => setTab("files")} icon={<FileCode2 size={16}/>} label="文件" />
        <ActivityButton active={tab === "diff"} onClick={() => setTab("diff")} icon={<GitCompareArrows size={16}/>} label="Git" />
        <ActivityButton active={tab === "terminal"} onClick={() => setTab("terminal")} icon={<SquareTerminal size={16}/>} label="终端" />
        <span className="activity-rail-spacer" />
        <ActivityButton active={terminalOpen} onClick={onToggleTerminal} icon={terminalOpen ? <PanelBottomClose size={16}/> : <PanelBottomOpen size={16}/>} label="底部终端" />
        <ActivityButton active={maximized} onClick={onToggleMaximized} icon={maximized ? <Minimize2 size={16}/> : <Maximize2 size={16}/>} label="编辑器全屏" />
        <ActivityButton active={false} onClick={onToggleSwapped} icon={<ArrowLeftRight size={16}/>} label="交换布局" />
        <ActivityButton active={false} onClick={onToggleChat} icon={<PanelLeftClose size={16}/>} label="聊天区" />
        <ActivityButton active={false} onClick={onClose} icon={<PanelRightClose size={16}/>} label="关闭工作区" />
      </nav>
    </div>
  );
}

/**
 * 渲染工作区标签按钮。
 *
 * @param props 标签状态、图标和点击回调
 * @returns 工作区标签按钮
 */
function ActivityButton({ active,onClick,icon,label }:{ active:boolean;onClick:()=>void;icon:React.ReactNode;label:string }) {
  return <button type="button" className={active ? "active" : ""} onClick={onClick} title={label} aria-label={label}>{icon}</button>;
}
