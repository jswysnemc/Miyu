import { PanelLeftClose, PanelLeftOpen, Plus, TerminalSquare, X } from "lucide-react";
import { useState } from "react";
import { TerminalPane } from "./terminal-pane";
import type { TerminalManager } from "./use-terminal-manager";

/**
 * 渲染多会话终端面板。
 *
 * @param props 关闭面板回调与终端管理器
 * @returns 终端工作区
 */
export function TerminalDock({ onClose, manager }: { onClose: () => void; manager: TerminalManager }) {
  const [tabsCollapsed, setTabsCollapsed] = useState(false);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState("");
  return (
    <section className={tabsCollapsed ? "terminal-dock tabs-collapsed" : "terminal-dock"}>
      <aside className="terminal-sidebar">
        <header className="terminal-dock-nav">
          {tabsCollapsed ? (
            <button type="button" onClick={() => setTabsCollapsed(false)} aria-label="展开终端标签" title="展开终端标签"><PanelLeftOpen size={14} /></button>
          ) : <>
          <button type="button" className="active" aria-label="终端" title="终端"><TerminalSquare size={14} /></button>
          <span className="terminal-nav-spacer" />
          <button type="button" onClick={() => void manager.createTerminal()} aria-label="新建终端" title="新建终端"><Plus size={14} /></button>
          <button type="button" onClick={() => setTabsCollapsed(true)} aria-label="折叠终端标签" title="折叠终端标签"><PanelLeftClose size={14} /></button>
          </>}
        </header>
        <div className="terminal-list">
          {manager.terminals.map((terminal, index) => (
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
        </div>
        <button type="button" className="terminal-dock-close" onClick={onClose}><X size={14} />{!tabsCollapsed && <span>关闭面板</span>}</button>
      </aside>
      <div className="terminal-main">
        {manager.activeId ? <TerminalPane terminalId={manager.activeId} /> : (
          <div className="terminal-empty"><TerminalSquare size={22} /><p>{manager.loading ? "正在读取终端" : "没有活动终端"}</p><button type="button" onClick={() => void manager.createTerminal()}>新建终端</button></div>
        )}
        {manager.error && <div className="pane-error terminal-error">{manager.error.message}</div>}
      </div>
    </section>
  );
}
