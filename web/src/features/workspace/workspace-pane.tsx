import { FileCode2, GitCompareArrows, TerminalSquare } from "lucide-react";
import { useState } from "react";
import { DiffPane } from "./diff-pane";
import { EditorPane } from "./editor-pane";
import { FileTree } from "./file-tree";
import { TerminalPane } from "../terminal/terminal-pane";
import "./workspace-pane.css";

type PaneTab = "files" | "diff" | "terminal";

export function WorkspacePane({ selectedFile, onSelectFile }: { selectedFile: string | null; onSelectFile: (path: string) => void }) {
  const [tab, setTab] = useState<PaneTab>("files");
  return (
    <div className="workspace-pane">
      <div className="pane-tabs" role="tablist" aria-label="工作区面板">
        <PaneButton active={tab === "files"} onClick={() => setTab("files")} icon={<FileCode2 size={15} />} label="文件" />
        <PaneButton active={tab === "diff"} onClick={() => setTab("diff")} icon={<GitCompareArrows size={15} />} label="变更" />
        <PaneButton active={tab === "terminal"} onClick={() => setTab("terminal")} icon={<TerminalSquare size={15} />} label="终端" />
      </div>
      <div className="pane-body">
        {tab === "files" && <div className="files-layout"><FileTree selectedFile={selectedFile} onSelectFile={onSelectFile} /><EditorPane path={selectedFile} /></div>}
        {tab === "diff" && <DiffPane />}
        {tab === "terminal" && <TerminalPane />}
      </div>
    </div>
  );
}

function PaneButton({ active, onClick, icon, label }: { active: boolean; onClick: () => void; icon: React.ReactNode; label: string }) {
  return <button type="button" role="tab" aria-selected={active} className={active ? "pane-tab active" : "pane-tab"} onClick={onClick}>{icon}<span>{label}</span></button>;
}
