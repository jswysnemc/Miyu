import { PanelBottomOpen, PanelLeftOpen, PanelRightOpen } from "lucide-react";
import type { CSSProperties } from "react";
import { ChatPage } from "../chat/chat-page";
import { SessionSidebar } from "../sessions/session-sidebar";
import { WorkspacePane } from "./workspace-pane";
import { WorkspaceResizeHandle } from "./workspace-resize-handle";
import { useWorkspaceLayout } from "./use-workspace-layout";
import { TerminalDock } from "../terminal/terminal-dock";
import { TerminalResizeHandle } from "../terminal/terminal-resize-handle";

type WorkspaceLayoutProps = {
  selectedFile: string | null;
  onSelectFile: (path: string) => void;
  onClearFile: () => void;
};

/**
 * 组合会话栏、聊天区和可调整的右侧工作区。
 *
 * @param props 文件选择状态与回调
 * @returns 编程工作区布局
 */
export function WorkspaceLayout({ selectedFile, onSelectFile, onClearFile }: WorkspaceLayoutProps) {
  const layout = useWorkspaceLayout();
  const style = {
    "--workspace-panel-width": `${layout.workspaceWidth}px`,
    "--terminal-panel-height": `${layout.terminalHeight}px`
  } as CSSProperties;
  const classes = [
    "coding-layout",
    layout.workspaceOpen ? "workspace-open" : "workspace-closed",
    layout.chatOpen ? "chat-open" : "chat-closed",
    layout.workspaceMaximized ? "workspace-maximized" : "",
    layout.terminalOpen ? "terminal-open" : "terminal-closed"
  ].filter(Boolean).join(" ");

  return (
    <div className={classes} style={style}>
      <aside className="coding-sidebar">
        <SessionSidebar />
      </aside>
      <div className="workbench-main">
        {layout.chatOpen && !layout.workspaceMaximized && <section className="coding-chat"><ChatPage /></section>}
        {layout.workspaceOpen && layout.chatOpen && !layout.workspaceMaximized && <WorkspaceResizeHandle onResize={layout.resizeWorkspace} />}
        {layout.workspaceOpen && (
          <aside className="coding-workspace">
            <WorkspacePane
              selectedFile={selectedFile}
              onSelectFile={onSelectFile}
              onClearFile={onClearFile}
              onClose={layout.closeWorkspace}
              onToggleChat={layout.toggleChat}
              onToggleMaximized={layout.toggleWorkspaceMaximized}
              onOpenTerminal={layout.openTerminal}
              maximized={layout.workspaceMaximized}
            />
          </aside>
        )}
        {!layout.workspaceOpen && (
          <div className="workbench-floating-controls">
            {!layout.chatOpen && <button type="button" onClick={layout.toggleChat}><PanelLeftOpen size={14} /><span>聊天</span></button>}
            <button type="button" onClick={layout.openWorkspace}><PanelRightOpen size={14} /><span>编辑器</span></button>
            {!layout.terminalOpen && <button type="button" onClick={layout.openTerminal}><PanelBottomOpen size={14} /><span>终端</span></button>}
          </div>
        )}
      </div>
      {layout.terminalOpen && (
        <div className="coding-terminal">
          <TerminalResizeHandle onResize={layout.resizeTerminal} />
          <TerminalDock onClose={layout.closeTerminal} />
        </div>
      )}
    </div>
  );
}
