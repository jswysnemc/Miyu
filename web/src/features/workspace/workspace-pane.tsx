import { ArrowLeftRight, FileCode2, GitBranch, GitCompareArrows, Maximize2, Minimize2, PanelBottomClose, PanelBottomOpen, PanelLeftClose, PanelRightClose } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "../../api/client";
import { DiffPane } from "./diff-pane";
import { EditorPane } from "./editor-pane";
import { FileTree } from "./file-tree";
import "./workspace-pane.css";

type PaneTab = "files" | "diff";

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
};

/**
 * 渲染文件、变更和终端工作区。
 *
 * @param props 文件选择状态和面板控制回调
 * @returns 右侧工作区面板
 */
export function WorkspacePane({ selectedFile, onSelectFile, onClearFile, onClose, onToggleChat, onToggleMaximized, onToggleTerminal, onToggleSwapped, terminalOpen, maximized }: WorkspacePaneProps) {
  const [tab, setTab] = useState<PaneTab>("files");
  const git = useQuery({ queryKey: ["workspace-diff"], queryFn: api.workspace.diff, staleTime: 20_000 });
  return (
    <div className="workspace-pane">
      <div className="pane-tabs" role="tablist" aria-label="工作区面板">
        <PaneButton active={tab === "files"} onClick={() => setTab("files")} icon={<FileCode2 size={15} />} label="文件" />
        <PaneButton active={tab === "diff"} onClick={() => setTab("diff")} icon={<GitCompareArrows size={15} />} label="变更" />
        {git.data?.repository && git.data.branch && (
          <span className="branch-chip" title={`当前分支 ${git.data.branch}`}><GitBranch size={12} />{git.data.branch}</span>
        )}
        <button type="button" className="pane-control" onClick={onToggleSwapped} aria-label="左右调换聊天与编辑器"><ArrowLeftRight size={15} /></button>
        <button type="button" className="pane-control" onClick={onToggleChat} aria-label="折叠或展开聊天区"><PanelLeftClose size={15} /></button>
        <button type="button" className="pane-control" onClick={onToggleTerminal} aria-label={terminalOpen ? "关闭底部终端" : "打开底部终端"}>{terminalOpen ? <PanelBottomClose size={15} /> : <PanelBottomOpen size={15} />}</button>
        <button type="button" className="pane-control" onClick={onToggleMaximized} aria-label={maximized ? "退出编辑器全屏" : "编辑器全屏"}>{maximized ? <Minimize2 size={15} /> : <Maximize2 size={15} />}</button>
        <button type="button" className="pane-close" onClick={onClose} aria-label="关闭工作区"><PanelRightClose size={15} /></button>
      </div>
      <div className="pane-body">
        {tab === "files" && <div className="files-layout"><FileTree selectedFile={selectedFile} onSelectFile={onSelectFile} onClearFile={onClearFile} /><EditorPane path={selectedFile} /></div>}
        {tab === "diff" && <DiffPane />}
      </div>
    </div>
  );
}

/**
 * 渲染工作区标签按钮。
 *
 * @param props 标签状态、图标和点击回调
 * @returns 工作区标签按钮
 */
function PaneButton({ active, onClick, icon, label }: { active: boolean; onClick: () => void; icon: React.ReactNode; label: string }) {
  return <button type="button" role="tab" aria-selected={active} className={active ? "pane-tab active" : "pane-tab"} onClick={onClick}>{icon}<span>{label}</span></button>;
}
