import { ArrowLeftRight, Bot, ChevronsLeft, ChevronsRight, FileCode2, GitCompareArrows, LayoutPanelLeft, Maximize2, MessageSquare, PanelBottomOpen, SlidersHorizontal, SquareTerminal } from "lucide-react";
import { useRef, useState } from "react";
import { useOutsidePointerDown } from "../../shared/hooks/use-outside-pointer-down";
import type { PaneTab } from "./workspace-tab";

type WorkspaceActivityRailProps = {
  tab: PaneTab;
  workspaceOpen: boolean;
  chatOpen: boolean;
  maximized: boolean;
  terminalOpen: boolean;
  onSelectTab: (tab: PaneTab) => void;
  onCollapse: () => void;
  onExpand: () => void;
  onToggleChat: () => void;
  onToggleMaximized: () => void;
  onToggleSwapped: () => void;
  onToggleTerminal: () => void;
};

const tabs: Array<{ id: PaneTab; label: string; icon: typeof FileCode2 }> = [
  { id: "files", label: "文件", icon: FileCode2 },
  { id: "diff", label: "Git", icon: GitCompareArrows },
  { id: "terminal", label: "终端", icon: SquareTerminal },
  { id: "subagents", label: "子智能体", icon: Bot }
];

/**
 * 渲染悬浮在工作台右缘的活动栏。
 *
 * 上段切换工作区视图:工作区关闭时点击任一图标打开并跳转,再次点击
 * 当前视图图标收起工作区。下段是布局弹出菜单(聊天区、全屏、交换、
 * 底部终端)和工作区收起/展开开关。
 *
 * @param props 当前视图、布局状态与各切换回调
 * @returns 悬浮活动栏
 */
export function WorkspaceActivityRail(props: WorkspaceActivityRailProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  useOutsidePointerDown(menuRef, () => setMenuOpen(false), menuOpen);

  /**
   * 处理视图图标点击:未打开则打开并切换,已打开的当前视图再点则收起。
   *
   * @param id 目标视图
   */
  const handleTab = (id: PaneTab) => {
    if (props.workspaceOpen && props.tab === id) {
      props.onCollapse();
      return;
    }
    props.onSelectTab(id);
  };

  return (
    <nav className="workspace-floating-rail" aria-label="工作区活动栏">
      {tabs.map(({ id, label, icon: Icon }) => {
        const active = props.workspaceOpen && props.tab === id;
        return (
          <button key={id} type="button" className={active ? "active" : ""} onClick={() => handleTab(id)} title={active ? `收起${label}` : label} aria-label={label} aria-pressed={active}>
            <Icon size={16} />
          </button>
        );
      })}
      <span className="rail-divider" aria-hidden />
      <div className="rail-menu-anchor" ref={menuRef}>
        <button type="button" className={menuOpen ? "active" : ""} onClick={() => setMenuOpen((value) => !value)} title="布局选项" aria-label="布局选项" aria-expanded={menuOpen}>
          <SlidersHorizontal size={15} />
        </button>
        {menuOpen && (
          <div className="rail-layout-menu" role="menu">
            <button type="button" role="menuitem" className={props.chatOpen ? "checked" : ""} onClick={props.onToggleChat}>
              <MessageSquare size={14} /><span>聊天区</span>
            </button>
            <button type="button" role="menuitem" className={props.maximized ? "checked" : ""} onClick={props.onToggleMaximized}>
              <Maximize2 size={14} /><span>工作区全屏</span>
            </button>
            <button type="button" role="menuitem" onClick={props.onToggleSwapped}>
              <ArrowLeftRight size={14} /><span>交换左右布局</span>
            </button>
            <button type="button" role="menuitem" className={props.terminalOpen ? "checked" : ""} onClick={props.onToggleTerminal}>
              <PanelBottomOpen size={14} /><span>底部终端</span>
            </button>
          </div>
        )}
      </div>
      <button
        type="button"
        onClick={props.workspaceOpen ? props.onCollapse : props.onExpand}
        title={props.workspaceOpen ? "收起工作区" : "展开工作区"}
        aria-label={props.workspaceOpen ? "收起工作区" : "展开工作区"}
      >
        {props.workspaceOpen ? <ChevronsRight size={16} /> : <ChevronsLeft size={16} />}
      </button>
      {!props.chatOpen && !props.workspaceOpen && (
        <button type="button" onClick={props.onToggleChat} title="显示聊天区" aria-label="显示聊天区">
          <LayoutPanelLeft size={16} />
        </button>
      )}
    </nav>
  );
}
