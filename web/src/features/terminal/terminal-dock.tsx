import { Activity, PanelBottomClose, Plus, TerminalSquare, X } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "../../api/client";
import { BackgroundTasksPanel } from "../background-tasks/background-tasks-panel";
import { TerminalPane } from "./terminal-pane";
import { useTerminalManager } from "./use-terminal-manager";

/**
 * 渲染底部多会话终端及左侧会话选择栏。
 *
 * @param props 关闭底部终端回调
 * @returns 底部终端工作区
 */
export function TerminalDock({ onClose }: { onClose: () => void }) {
  const manager = useTerminalManager();
  const [view, setView] = useState<"terminal" | "tasks">("terminal");
  const backgroundTasks = useQuery({ queryKey: ["background-tasks"], queryFn: api.backgroundTasks.list, refetchInterval: 3000 });
  const runningCount = backgroundTasks.data?.tasks.filter((task) => task.status === "running").length ?? 0;
  return (
    <section className="terminal-dock">
      <aside className="terminal-sidebar">
        <header className="terminal-dock-nav">
          <button type="button" className={view === "terminal" ? "active" : ""} onClick={() => setView("terminal")}><TerminalSquare size={13} /><span>终端</span></button>
          <button type="button" className={view === "tasks" ? "active" : ""} onClick={() => setView("tasks")}><Activity size={13} /><span>后台任务</span>{runningCount > 0 && <b>{runningCount}</b>}</button>
          {view === "terminal" && <button type="button" onClick={() => void manager.createTerminal()} aria-label="新建终端"><Plus size={14} /></button>}
        </header>
        <div className={`terminal-list${view === "tasks" ? " tasks-view" : ""}`}>
          {view === "terminal" && manager.terminals.map((terminal, index) => (
            <div className={terminal.id === manager.activeId ? "terminal-list-item active" : "terminal-list-item"} key={terminal.id}>
              <button type="button" onClick={() => manager.setActiveId(terminal.id)}><TerminalSquare size={13} /><span>{terminal.title || `终端 ${index + 1}`}</span></button>
              <button type="button" onClick={() => void manager.closeTerminal(terminal.id)} aria-label={`关闭终端 ${index + 1}`}><X size={12} /></button>
            </div>
          ))}
          {view === "tasks" && <p className="terminal-task-summary">{runningCount > 0 ? `${runningCount} 个任务正在运行` : "没有运行中的任务"}</p>}
        </div>
        <button type="button" className="terminal-dock-close" onClick={onClose}><PanelBottomClose size={14} /><span>收起终端</span></button>
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
