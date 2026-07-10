import { useQuery } from "@tanstack/react-query";
import { RefreshCw } from "lucide-react";
import { api } from "../../api/client";

export function DiffPane() {
  const diff = useQuery({ queryKey: ["workspace-diff"], queryFn: api.workspace.diff });
  return (
    <section className="diff-pane">
      <header className="panel-head"><div><span className="eyebrow">Git working tree</span><h2>未提交变更</h2></div><button type="button" className="icon-button" onClick={() => void diff.refetch()} aria-label="刷新变更"><RefreshCw size={14} /></button></header>
      {!diff.data?.repository && !diff.isLoading && <div className="editor-empty"><p>当前工作区不是 Git 仓库</p></div>}
      {diff.data?.repository && (
        <div className="diff-scroll">
          <pre className="git-status">{diff.data.status || "工作区没有变更"}</pre>
          {diff.data.diff && <div className="workspace-diff">{diff.data.diff.split("\n").map((line, index) => <div className={line.startsWith("+") && !line.startsWith("+++") ? "diff-line added" : line.startsWith("-") && !line.startsWith("---") ? "diff-line removed" : line.startsWith("@@") ? "diff-line hunk" : "diff-line"} key={`${index}-${line}`}>{line || " "}</div>)}</div>}
        </div>
      )}
      {diff.error && <div className="pane-error">{diff.error.message}</div>}
    </section>
  );
}
