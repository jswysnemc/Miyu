import { Activity, PanelBottomClose, PanelLeftClose, PanelLeftOpen, Plus, TerminalSquare, X } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "../../api/client";
import { BackgroundTasksPanel } from "../background-tasks/background-tasks-panel";
import { TerminalPane } from "./terminal-pane";
import type { TerminalManager } from "./use-terminal-manager";

/**
 * 渲染底部多会话终端及左侧会话选择栏。
 *
 * @param props 关闭底部终端回调
 * @returns 底部终端工作区
 */
export function TerminalDock({ onClose, manager }: { onClose: () => void; manager: TerminalManager }) {
  const [view, setView] = useState<"terminal" | "tasks">("terminal");
  const [tabsCollapsed, setTabsCollapsed] = useState(false);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState("");
  const backgroundTasks = useQuery({ queryKey: ["background-tasks"], queryFn: api.backgroundTasks.list, refetchInterval: 3000 });
  const runningCount = backgroundTasks.data?.tasks.filter((task) => task.status === "running").length ?? 0;
  return (
    <section className={tabsCollapsed ? "terminal-dock tabs-collapsed" : "terminal-dock"}>
      <aside className="terminal-sidebar">
        <header className="terminal-dock-nav">
          {tabsCollapsed ? (
            <button type="button" onClick={() => setTabsCollapsed(false)} aria-label="展开终端标签" title="展开终端标签"><PanelLeftOpen size={14} /></button>
          ) : <>
          <button type="button" className={view === "terminal" ? "active" : ""} onClick={() => setView("terminal")} aria-label="终端" title="终端"><TerminalSquare size={14} /></button>
          <button type="button" className={view === "tasks" ? "nav-badge-anchor active" : "nav-badge-anchor"} onClick={() => setView("tasks")} aria-label="后台任务" title="后台任务"><Activity size={14} />{runningCount > 0 && <b>{runningCount}</b>}</button>
          <span className="terminal-nav-spacer" />
          {view === "terminal" && <button type="button" onClick={() => void manager.createTerminal()} aria-label="新建终端" title="新建终端"><Plus size={14} /></button>}
          <button type="button" onClick={() => setTabsCollapsed(true)} aria-label="折叠终端标签" title="折叠终端标签"><PanelLeftClose size={14} /></button>
          </>}
        </header>
        <div className={`terminal-list${view === "tasks" ? " tasks-view" : ""}`}>
          {view === "terminal" && manager.terminals.map((terminal, index) => (
            <div className={terminal.id === manager.activeId ? "terminal-list-item active" : "terminal-list-item"} key={terminal.id}>
              {renamingId === terminal.id ? (
                <input
                  autoFocus
                  className="terminal-title-input"
                  value={renameDraft}
                  onChange={(event) => setRenameDraft(event.target.value)}
                  onBlur={() => setRenamingId(null)}
                  onKeyDown={(event) => {
                    if (event.key === "Escape") setRenamingId(null);
                    if (event.key === "Enter" && renameDraft.trim()) {
                      void manager.renameTerminal(terminal.id, renameDraft.trim());
                      setRenamingId(null);
                    }
                  }}
                  aria-label={`重命名终端 ${index + 1}`}
                />
              ) : (
                <button type="button" onClick={() => manager.setActiveId(terminal.id)} onDoubleClick={() => { setRenamingId(terminal.id); setRenameDraft(terminal.title || `终端 ${index + 1}`); }}><TerminalSquare size={13} /><span>{terminal.title || `终端 ${index + 1}`}</span></button>
              )}
              <button type="button" onClick={() => void manager.closeTerminal(terminal.id)} aria-label={`关闭终端 ${index + 1}`}><X size={12} /></button>
            </div>
          ))}
          {view === "tasks" && <p className="terminal-task-summary">{runningCount > 0 ? `${runningCount} 个任务正在运行` : "没有运行中的任务"}</p>}
        </div>
        <button type="button" className="terminal-dock-close" onClick={onClose}><PanelBottomClose size={14} />{!tabsCollapsed && <span>收起终端</span>}</button>
      </aside>
      <div className="terminal-main">
        {view === "tasks" ? <BackgroundTasksPanel /> : manager.activeId ? <TerminalPane terminalId={manager.activeId} /> : (
          <div className="terminal-empty"><TerminalSquare size={22} /><p>{manager.loading ? "正在读取终端" : "没有活动终端"}</p><button type="button" onClick={() => void manager.createTerminal()}>新建终端</button></div>
        )}
        {manager.error && <div className="pane-error terminal-error">{manager.error.message}</div>}
      </div>
    </section>
  );
}
