import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, GitBranch, Minus, Plus, RefreshCw, RotateCcw, Trash2 } from "lucide-react";
import { useState } from "react";
import { api } from "../../api/client";
import type { GitFileStatus } from "../../api/contracts";

/**
 * 渲染 Git 状态、暂存操作、提交输入和完整差异。
 *
 * @returns Git 管理面板
 */
export function DiffPane() {
  const queryClient = useQueryClient();
  const git = useQuery({ queryKey: ["workspace-diff"], queryFn: api.workspace.diff });
  const [message, setMessage] = useState("");
  const [pending, setPending] = useState("");
  const [error, setError] = useState("");

  /** 执行 Git 操作并刷新文件树。 */
  const act = async (action: "stage" | "unstage" | "discard" | "commit", paths: string[] = []) => {
    if (action === "discard" && !window.confirm(`确定撤销 ${paths.join("、")} 的工作区修改吗？`)) return;
    setPending(`${action}:${paths.join("|")}`);
    setError("");
    try {
      const next = await api.workspace.gitAction(action, paths, action === "commit" ? message : undefined);
      queryClient.setQueryData(["workspace-diff"], next);
      if (action === "commit") setMessage("");
      await queryClient.invalidateQueries({ queryKey: ["file-tree"] });
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setPending("");
    }
  };

  /** 删除未跟踪文件。 */
  const removeUntracked = async (path: string) => {
    if (!window.confirm(`确定删除未跟踪文件“${path}”吗？`)) return;
    setPending(`delete:${path}`);
    try {
      await api.workspace.remove(path);
      await Promise.all([git.refetch(), queryClient.invalidateQueries({ queryKey: ["file-tree"] })]);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setPending("");
    }
  };

  return (
    <section className="diff-pane git-manager">
      <header className="panel-head">
        <div><span className="eyebrow">Git 工作区</span><h2><GitBranch size={15} />{git.data?.branch || "版本管理"}</h2></div>
        <button type="button" className="icon-button" onClick={() => void git.refetch()} aria-label="刷新变更"><RefreshCw size={14} /></button>
      </header>
      {!git.data?.repository && !git.isLoading && <div className="editor-empty"><p>当前工作区不是 Git 仓库</p></div>}
      {git.data?.repository && (
        <div className="git-manager-body">
          <section className="git-change-panel">
            <div className="git-commit-box"><textarea rows={3} value={message} onChange={(event) => setMessage(event.target.value)} placeholder="提交说明" /><button type="button" onClick={() => void act("commit")} disabled={!message.trim() || Boolean(pending)}><Check size={13} />提交已暂存变更</button></div>
            <div className="git-change-head"><span>变更 {git.data.files.length}</span><span><button type="button" onClick={() => void act("unstage")} title="取消全部暂存"><Minus size={12} /></button><button type="button" onClick={() => void act("stage")} title="暂存全部"><Plus size={12} /></button></span></div>
            <div className="git-file-list">
              {git.data.files.map((file) => <GitFileRow file={file} pending={pending} onAction={act} onRemoveUntracked={removeUntracked} key={`${file.index_status}${file.worktree_status}${file.path}`} />)}
              {git.data.files.length === 0 && <div className="git-clean">工作区没有变更</div>}
            </div>
          </section>
          <div className="diff-scroll">
            {git.data.diff ? <div className="workspace-diff">{git.data.diff.split("\n").map((line, index) => <div className={diffLineClass(line)} key={`${index}-${line}`}>{line || " "}</div>)}</div> : <div className="git-clean diff-clean">没有可显示的差异</div>}
          </div>
        </div>
      )}
      {(git.error || error) && <div className="pane-error">{error || git.error?.message}</div>}
    </section>
  );
}

/** 渲染单个 Git 文件及可用操作。 */
function GitFileRow({ file, pending, onAction, onRemoveUntracked }: { file: GitFileStatus; pending: string; onAction: (action: "stage" | "unstage" | "discard", paths?: string[]) => Promise<void>; onRemoveUntracked: (path: string) => Promise<void> }) {
  const untracked = file.index_status === "?" && file.worktree_status === "?";
  const staged = file.index_status !== " " && file.index_status !== "?";
  const changed = file.worktree_status !== " " || untracked;
  return (
    <div className="git-file-row">
      <span className="git-file-status">{`${file.index_status}${file.worktree_status}`}</span><span title={file.path}>{file.path}</span>
      <span className="git-file-actions">
        {staged && <button type="button" disabled={Boolean(pending)} onClick={() => void onAction("unstage", [file.path])} title="取消暂存"><Minus size={12} /></button>}
        {changed && <button type="button" disabled={Boolean(pending)} onClick={() => void onAction("stage", [file.path])} title="暂存"><Plus size={12} /></button>}
        {changed && !untracked && <button type="button" disabled={Boolean(pending)} onClick={() => void onAction("discard", [file.path])} title="撤销修改"><RotateCcw size={12} /></button>}
        {untracked && <button type="button" disabled={Boolean(pending)} onClick={() => void onRemoveUntracked(file.path)} title="删除未跟踪文件"><Trash2 size={12} /></button>}
      </span>
    </div>
  );
}

/** 返回差异行样式。 */
function diffLineClass(line: string): string {
  if (line.startsWith("+") && !line.startsWith("+++")) return "diff-line added";
  if (line.startsWith("-") && !line.startsWith("---")) return "diff-line removed";
  if (line.startsWith("@@")) return "diff-line hunk";
  if (line.startsWith("# ")) return "diff-line section";
  return "diff-line";
}
