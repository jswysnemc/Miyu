import { Code2, MessageSquare, SquareTerminal } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import type { CSSProperties } from "react";
import { useEffect, useReducer, useState } from "react";
import { api } from "../../api/client";
import { ChatPage } from "../chat/chat-page";
import { SessionSidebar } from "../sessions/session-sidebar";
import { SessionSidebarResizeHandle } from "../sessions/session-sidebar-resize-handle";
import { useSessionSidebarLayout } from "../sessions/use-session-sidebar-layout";
import { WorkspaceActivityRail } from "./workspace-activity-rail";
import { WorkspacePane } from "./workspace-pane";
import { WorkspaceResizeHandle } from "./workspace-resize-handle";
import { useWorkspaceLayout } from "./use-workspace-layout";
import { workspaceRelativePath } from "./workspace-path-utils";
import type { PaneTab } from "./workspace-tab";
import { TerminalDock } from "../terminal/terminal-dock";
import { TerminalResizeHandle } from "../terminal/terminal-resize-handle";
import { useTerminalManager } from "../terminal/use-terminal-manager";
import {
  initialMobileWorkbenchState,
  MOBILE_SIDEBAR_TOGGLE_EVENT,
  MOBILE_WORKBENCH_MEDIA_QUERY,
  reduceMobileWorkbenchState,
  type MobileWorkbenchPane
} from "./mobile-workbench-state";

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
  const terminalManager = useTerminalManager();
  const sessionSidebar = useSessionSidebarLayout();
  const [paneTab, setPaneTab] = useState<PaneTab>("files");
  const [mobileLayout, dispatchMobileLayout] = useReducer(reduceMobileWorkbenchState, initialMobileWorkbenchState);
  const [isMobile, setIsMobile] = useState(() => window.matchMedia(MOBILE_WORKBENCH_MEDIA_QUERY).matches);
  const workspaces = useQuery({ queryKey: ["workspaces"], queryFn: api.workspaces.list });
  const activeWorkspace = workspaces.data?.workspaces.find((workspace) => workspace.id === workspaces.data.active_id);
  const style = {
    "--session-sidebar-width": `${sessionSidebar.width}px`,
    "--workspace-panel-width": `${layout.workspaceWidth}px`,
    "--terminal-panel-height": `${layout.terminalHeight}px`
  } as CSSProperties;
  const classes = [
    "coding-layout",
    layout.workspaceOpen ? "workspace-open" : "workspace-closed",
    layout.chatOpen ? "chat-open" : "chat-closed",
    layout.workspaceMaximized ? "workspace-maximized" : "",
    layout.terminalOpen ? "terminal-open" : "terminal-closed",
    layout.swapped ? "layout-swapped" : "",
    sessionSidebar.collapsed ? "sidebar-collapsed" : "sidebar-expanded",
    mobileLayout.sidebarOpen ? "mobile-sidebar-open" : "mobile-sidebar-closed",
    `mobile-pane-${mobileLayout.pane}`
  ].filter(Boolean).join(" ");

  useEffect(() => {
    const media = window.matchMedia(MOBILE_WORKBENCH_MEDIA_QUERY);
    /**
     * 同步当前视口是否使用移动端工作台。
     *
     * @param event 媒体查询变化事件
     */
    const handleMobileChange = (event: MediaQueryListEvent) => setIsMobile(event.matches);
    setIsMobile(media.matches);
    media.addEventListener("change", handleMobileChange);
    return () => media.removeEventListener("change", handleMobileChange);
  }, []);

  useEffect(() => {
    /** 响应顶部 Logo 发出的移动端会话侧栏切换事件。 */
    const handleSidebarToggle = () => {
      if (!window.matchMedia(MOBILE_WORKBENCH_MEDIA_QUERY).matches) return;
      if (!mobileLayout.sidebarOpen) sessionSidebar.expand();
      dispatchMobileLayout({ type: "toggle-sidebar" });
    };
    window.addEventListener(MOBILE_SIDEBAR_TOGGLE_EVENT, handleSidebarToggle);
    return () => window.removeEventListener(MOBILE_SIDEBAR_TOGGLE_EVENT, handleSidebarToggle);
  }, [mobileLayout.sidebarOpen, sessionSidebar.expand]);

  useEffect(() => {
    /** 响应消息区发出的"在编辑器中打开文件"事件。 */
    const handleOpenFile = (event: Event) => {
      const path = (event as CustomEvent<{ path?: string }>).detail?.path;
      if (!path) return;
      // 1. 选中文件并确保右侧工作区可见
      onSelectFile(workspaceRelativePath(path, activeWorkspace?.path ?? ""));
      layout.openWorkspace();
      if (window.matchMedia(MOBILE_WORKBENCH_MEDIA_QUERY).matches) {
        dispatchMobileLayout({ type: "show-pane", pane: "workspace" });
      }
    };
    window.addEventListener("miyu:open-file", handleOpenFile);
    return () => window.removeEventListener("miyu:open-file", handleOpenFile);
  }, [activeWorkspace?.path, onSelectFile, layout.openWorkspace]);

  useEffect(() => {
    const handleToggleTerminal = () => layout.toggleTerminal();
    window.addEventListener("miyu:toggle-terminal", handleToggleTerminal);
    return () => window.removeEventListener("miyu:toggle-terminal", handleToggleTerminal);
  }, [layout.toggleTerminal]);

  useEffect(() => {
    // 响应聊天区"查看子智能体"请求,打开右侧工作区并切到子智能体视图
    const handleOpenSubagents = () => {
      layout.openWorkspace();
      setPaneTab("subagents");
      if (window.matchMedia(MOBILE_WORKBENCH_MEDIA_QUERY).matches) {
        dispatchMobileLayout({ type: "show-pane", pane: "workspace" });
      }
    };
    window.addEventListener("miyu:open-subagents", handleOpenSubagents);
    return () => window.removeEventListener("miyu:open-subagents", handleOpenSubagents);
  }, [layout.openWorkspace]);

  /**
   * 显示指定移动端工作台面板，并确保对应桌面布局区域已经打开。
   *
   * @param pane 需要显示的面板
   */
  const showMobilePane = (pane: MobileWorkbenchPane) => {
    if (pane === "chat" && !layout.chatOpen) layout.toggleChat();
    if (pane === "chat" && layout.workspaceMaximized) layout.toggleWorkspaceMaximized();
    if (pane === "workspace") layout.openWorkspace();
    if (pane === "terminal") layout.openTerminal();
    dispatchMobileLayout({ type: "show-pane", pane });
  };

  /** 关闭编辑器，并在移动端回到聊天面板。 */
  const closeWorkspace = () => {
    layout.closeWorkspace();
    dispatchMobileLayout({ type: "show-pane", pane: "chat" });
  };

  /** 关闭终端，并在移动端回到聊天面板。 */
  const closeTerminal = () => {
    layout.closeTerminal();
    dispatchMobileLayout({ type: "show-pane", pane: "chat" });
  };

  /**
   * 打开工作区并切换到指定视图,悬浮活动栏使用。
   *
   * @param tab 目标工作区视图
   */
  const showWorkspaceTab = (tab: PaneTab) => {
    layout.openWorkspace();
    setPaneTab(tab);
  };

  return (
    <div className={classes} style={style}>
      <nav className="workbench-mobile-tabs" aria-label="工作台面板">
        <button type="button" className={mobileLayout.pane === "chat" ? "active" : ""} onClick={() => showMobilePane("chat")} aria-current={mobileLayout.pane === "chat" ? "page" : undefined}>
          <MessageSquare size={17} /><span>聊天</span>
        </button>
        <button type="button" className={mobileLayout.pane === "workspace" ? "active" : ""} onClick={() => showMobilePane("workspace")} aria-current={mobileLayout.pane === "workspace" ? "page" : undefined}>
          <Code2 size={17} /><span>编辑器</span>
        </button>
        <button type="button" className={mobileLayout.pane === "terminal" ? "active" : ""} onClick={() => showMobilePane("terminal")} aria-current={mobileLayout.pane === "terminal" ? "page" : undefined}>
          <SquareTerminal size={17} /><span>终端</span>
        </button>
      </nav>
      {mobileLayout.sidebarOpen && (
        <button type="button" className="mobile-sidebar-scrim" onClick={() => dispatchMobileLayout({ type: "close-sidebar" })} aria-label="关闭会话侧栏" />
      )}
      <aside className="coding-sidebar" aria-hidden={isMobile && !mobileLayout.sidebarOpen} inert={isMobile && !mobileLayout.sidebarOpen}>
        <SessionSidebar
          collapsed={sessionSidebar.collapsed}
          onToggleCollapsed={sessionSidebar.toggleCollapsed}
          onNavigate={() => dispatchMobileLayout({ type: "close-sidebar" })}
        />
        {!sessionSidebar.collapsed && <SessionSidebarResizeHandle width={sessionSidebar.width} onResize={sessionSidebar.resize} />}
      </aside>
      <div className="workbench-main">
        {layout.chatOpen && !layout.workspaceMaximized && <section className="coding-chat"><ChatPage /></section>}
        {layout.workspaceOpen && layout.chatOpen && !layout.workspaceMaximized && <WorkspaceResizeHandle swapped={layout.swapped} onResize={layout.resizeWorkspace} />}
        {layout.workspaceOpen && (
          <aside className="coding-workspace">
            <WorkspacePane
              selectedFile={selectedFile}
              tab={paneTab}
              onTabChange={setPaneTab}
              onSelectFile={onSelectFile}
              onClearFile={onClearFile}
              terminalManager={terminalManager}
            />
          </aside>
        )}
        <WorkspaceActivityRail
          tab={paneTab}
          workspaceOpen={layout.workspaceOpen}
          chatOpen={layout.chatOpen}
          maximized={layout.workspaceMaximized}
          terminalOpen={layout.terminalOpen}
          onSelectTab={showWorkspaceTab}
          onCollapse={closeWorkspace}
          onExpand={layout.openWorkspace}
          onToggleChat={layout.toggleChat}
          onToggleMaximized={layout.toggleWorkspaceMaximized}
          onToggleSwapped={layout.toggleSwapped}
          onToggleTerminal={layout.toggleTerminal}
        />
      </div>
      {layout.terminalOpen && (
        <div className="coding-terminal">
          <TerminalResizeHandle onResize={layout.resizeTerminal} />
          <TerminalDock manager={terminalManager} onClose={closeTerminal} />
        </div>
      )}
    </div>
  );
}
