import { PanelBottomOpen, PanelLeftOpen, PanelRightOpen } from "lucide-react";
import type { CSSProperties } from "react";
import { useEffect } from "react";
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
    layout.terminalOpen ? "terminal-open" : "terminal-closed",
    layout.swapped ? "layout-swapped" : ""
  ].filter(Boolean).join(" ");

  useEffect(() => {
    /** 响应消息区发出的"在编辑器中打开文件"事件。 */
    const handleOpenFile = (event: Event) => {
      const path = (event as CustomEvent<{ path?: string }>).detail?.path;
      if (!path) return;
      // 1. 选中文件并确保右侧工作区可见
      onSelectFile(path);
      layout.openWorkspace();
    };
    window.addEventListener("miyu:open-file", handleOpenFile);
    return () => window.removeEventListener("miyu:open-file", handleOpenFile);
  }, [onSelectFile, layout.openWorkspace]);

  return (
    <div className={classes} style={style}>
      <aside className="coding-sidebar">
        <SessionSidebar />
      </aside>
      <div className="workbench-main">
        {layout.chatOpen && !layout.workspaceMaximized && <section className="coding-chat"><ChatPage /></section>}
        {layout.workspaceOpen && layout.chatOpen && !layout.workspaceMaximized && <WorkspaceResizeHandle swapped={layout.swapped} onResize={layout.resizeWorkspace} />}
        {layout.workspaceOpen && (
          <aside className="coding-workspace">
            <WorkspacePane
              selectedFile={selectedFile}
              onSelectFile={onSelectFile}
              onClearFile={onClearFile}
              onClose={layout.closeWorkspace}
              onToggleChat={layout.toggleChat}
              onToggleMaximized={layout.toggleWorkspaceMaximized}
              onToggleTerminal={layout.toggleTerminal}
              onToggleSwapped={layout.toggleSwapped}
              terminalOpen={layout.terminalOpen}
              maximized={layout.workspaceMaximized}
            />
          </aside>
        )}
        {!layout.workspaceOpen && (
          <div className="workbench-floating-controls">
            {!layout.chatOpen && <button type="button" onClick={layout.toggleChat}><PanelLeftOpen size={14} /><span>聊天</span></button>}
            <button type="button" onClick={layout.openWorkspace}><PanelRightOpen size={14} /><span>编辑器</span></button>
            <button type="button" onClick={layout.toggleTerminal}><PanelBottomOpen size={14} /><span>终端</span></button>
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
