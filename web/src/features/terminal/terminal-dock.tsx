import { PanelBottomClose, Plus, TerminalSquare, X } from "lucide-react";
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
  return (
    <section className="terminal-dock">
      <aside className="terminal-sidebar">
        <header><span>终端</span><button type="button" onClick={() => void manager.createTerminal()} aria-label="新建终端"><Plus size={14} /></button></header>
        <div className="terminal-list">
          {manager.terminals.map((terminal, index) => (
            <div className={terminal.id === manager.activeId ? "terminal-list-item active" : "terminal-list-item"} key={terminal.id}>
              <button type="button" onClick={() => manager.setActiveId(terminal.id)}><TerminalSquare size={13} /><span>{terminal.title || `终端 ${index + 1}`}</span></button>
              <button type="button" onClick={() => void manager.closeTerminal(terminal.id)} aria-label={`关闭终端 ${index + 1}`}><X size={12} /></button>
            </div>
          ))}
        </div>
        <button type="button" className="terminal-dock-close" onClick={onClose}><PanelBottomClose size={14} /><span>收起终端</span></button>
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
