import { Bot, FileCode2, GitCompareArrows, Plus, SquareTerminal, X } from "lucide-react";
import { useRef, useState } from "react";
import { useOutsidePointerDown } from "../../shared/hooks/use-outside-pointer-down";
import type { PaneTab, WorkspacePanelTab } from "./workspace-tab";
import { paneTabLabel } from "./workspace-tab";

type WorkspaceTabBarProps = {
  tabs: WorkspacePanelTab[];
  activeTabId: string | null;
  onActivate: (id: string) => void;
  onClose: (id: string) => void;
  onAdd: (type: PaneTab) => void;
};

const addable: Array<{ type: PaneTab; label: string; icon: typeof FileCode2 }> = [
  { type: "files", label: "编辑器", icon: FileCode2 },
  { type: "diff", label: "Git", icon: GitCompareArrows },
  { type: "terminal", label: "终端", icon: SquareTerminal },
  { type: "subagents", label: "子智能体", icon: Bot }
];

/**
 * 渲染 Cursor 风格的工作区顶部标签栏。
 *
 * @param props 标签列表、当前标签与增删回调
 * @returns 工作区标签导航
 */
export function WorkspaceTabBar(props: WorkspaceTabBarProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  useOutsidePointerDown(menuRef, () => setMenuOpen(false), menuOpen);

  return (
    <div className="workspace-tab-bar" role="tablist" aria-label="工作区标签">
      <div className="workspace-tab-scroll">
        {props.tabs.map((tab) => {
          const active = tab.id === props.activeTabId;
          return (
            <div key={tab.id} className={active ? "workspace-tab active" : "workspace-tab"} role="presentation">
              <button
                type="button"
                role="tab"
                aria-selected={active}
                className="workspace-tab-main"
                onClick={() => props.onActivate(tab.id)}
                title={tab.path ?? tab.title}
              >
                <TabIcon type={tab.type} />
                <span>{tab.title || paneTabLabel(tab.type)}</span>
              </button>
              {tab.closable && (
                <button
                  type="button"
                  className="workspace-tab-close"
                  aria-label={`关闭 ${tab.title}`}
                  onClick={(event) => {
                    event.stopPropagation();
                    props.onClose(tab.id);
                  }}
                >
                  <X size={12} />
                </button>
              )}
            </div>
          );
        })}
      </div>
      <div className="workspace-tab-actions" ref={menuRef}>
        <button
          type="button"
          className="workspace-tab-add"
          aria-label="添加面板"
          title="添加面板"
          aria-expanded={menuOpen}
          onClick={() => setMenuOpen((value) => !value)}
        >
          <Plus size={14} />
        </button>
        {menuOpen && (
          <div className="workspace-tab-add-menu" role="menu">
            {addable.map((item) => {
              const Icon = item.icon;
              return (
                <button
                  type="button"
                  role="menuitem"
                  key={item.type}
                  onClick={() => {
                    props.onAdd(item.type);
                    setMenuOpen(false);
                  }}
                >
                  <Icon size={14} />
                  <span>{item.label}</span>
                </button>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

function TabIcon({ type }: { type: PaneTab }) {
  if (type === "diff") return <GitCompareArrows size={13} />;
  if (type === "terminal") return <SquareTerminal size={13} />;
  if (type === "subagents") return <Bot size={13} />;
  return <FileCode2 size={13} />;
}
