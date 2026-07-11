import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckSquare2, ListChecks, MessageSquare, MoreHorizontal, PanelLeftClose, PanelLeftOpen, Pencil, Plus, RefreshCw, Settings, Square, Trash2, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { NavLink } from "react-router-dom";
import { api } from "../../api/client";
import "./session-sidebar.css";

type SessionSidebarProps = {
  collapsed: boolean;
  onToggleCollapsed: () => void;
  onNavigate?: () => void;
};

/**
 * 渲染会话列表、新建入口和批量管理模式。
 *
 * @param props 折叠状态和切换回调
 * @returns 会话侧栏
 */
export function SessionSidebar({ collapsed, onToggleCollapsed, onNavigate }: SessionSidebarProps) {
  const queryClient = useQueryClient();
  const [menu, setMenu] = useState<string | null>(null);
  const [selecting, setSelecting] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState(false);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState("");
  const menuRef = useRef<HTMLDivElement | null>(null);
  const sessions = useQuery({ queryKey: ["sessions"], queryFn: api.sessions.list });

  // 1. 监听整页 pointerdown，点击菜单外任意位置时关闭管理菜单
  useEffect(() => {
    if (!menu) return;
    /**
     * 处理菜单外部点击并关闭菜单。
     *
     * @param event 指针事件
     */
    const onPointerDown = (event: PointerEvent) => {
      if (menuRef.current && event.target instanceof Node && menuRef.current.contains(event.target)) return;
      setMenu(null);
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [menu]);

  /** 刷新会话列表和全部消息缓存。 */
  const refresh = async () => {
    await queryClient.invalidateQueries({ queryKey: ["sessions"] });
    await queryClient.invalidateQueries({ queryKey: ["messages"] });
    await queryClient.invalidateQueries({ queryKey: ["timeline"] });
  };

  const create = useMutation({ mutationFn: () => api.sessions.create(), onSuccess: refresh });
  const switchSession = useMutation({ mutationFn: api.sessions.switch, onSuccess: refresh });
  const remove = useMutation({ mutationFn: api.sessions.remove, onSuccess: refresh });
  const rename = useMutation({
    mutationFn: ({ id, title }: { id: string; title: string }) => api.sessions.rename(id, title),
    onSuccess: async () => {
      setRenaming(null);
      await refresh();
    }
  });
  const removeMany = useMutation({
    mutationFn: api.sessions.removeMany,
    onSuccess: async () => {
      setSelected(new Set());
      setSelecting(false);
      setConfirming(false);
      await refresh();
    }
  });

  /**
   * 切换指定会话的选中状态。
   *
   * @param id 会话 ID
   */
  const toggleSelected = (id: string) => {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
    setConfirming(false);
  };

  /** 切换全部可删除会话的选中状态。 */
  const toggleAll = () => {
    const ids = sessions.data?.filter((session) => session.id !== "default").map((session) => session.id) ?? [];
    setSelected(selected.size === ids.length ? new Set() : new Set(ids));
    setConfirming(false);
  };

  /** 执行两阶段批量删除确认。 */
  const requestBulkDelete = () => {
    if (selected.size === 0) return;
    if (!confirming) {
      setConfirming(true);
      return;
    }
    removeMany.mutate(Array.from(selected));
  };

  /** 退出选择模式并清理临时状态。 */
  const closeSelection = () => {
    setSelecting(false);
    setSelected(new Set());
    setConfirming(false);
  };

  /**
   * 进入指定会话的重命名编辑态。
   *
   * @param id 会话 ID
   * @param title 当前标题
   */
  const startRename = (id: string, title: string) => {
    setRenaming(id);
    setRenameDraft(title);
    setMenu(null);
  };

  /** 提交重命名，标题为空或未变化时直接退出编辑态。 */
  const submitRename = () => {
    if (!renaming) return;
    const title = renameDraft.trim();
    const current = sessions.data?.find((session) => session.id === renaming);
    if (!title || title === current?.title) {
      setRenaming(null);
      return;
    }
    rename.mutate({ id: renaming, title });
  };

  const manageableCount = sessions.data?.filter((session) => session.id !== "default").length ?? 0;
  const error = sessions.error ?? switchSession.error ?? remove.error ?? removeMany.error ?? rename.error;

  if (collapsed) {
    return (
      <div className="session-sidebar collapsed">
        <button type="button" className="sidebar-rail-button" onClick={onToggleCollapsed} aria-label="展开会话侧栏" title="展开会话侧栏">
          <PanelLeftOpen size={17} />
        </button>
        <button type="button" className="sidebar-rail-button" onClick={() => create.mutate()} disabled={create.isPending} aria-label="新建会话" title="新建会话">
          <Plus size={17} />
        </button>
        <NavLink to="/settings" onClick={onNavigate} className={({ isActive }) => isActive ? "sidebar-rail-button active" : "sidebar-rail-button"} aria-label="打开配置" title="配置">
          <Settings size={17} strokeWidth={1.8} />
        </NavLink>
      </div>
    );
  }

  return (
    <div className="session-sidebar">
      <div className="sidebar-heading">
        <div><span className="eyebrow">Sessions</span><h2>会话</h2></div>
        <div className="sidebar-heading-actions">
          <button type="button" className="icon-button" aria-label="折叠会话侧栏" title="折叠会话侧栏" onClick={onToggleCollapsed}>
            <PanelLeftClose size={16} />
          </button>
          <button type="button" className={selecting ? "icon-button active" : "icon-button"} aria-label={selecting ? "退出选择" : "批量管理"} onClick={() => selecting ? closeSelection() : setSelecting(true)} disabled={manageableCount === 0}>
            {selecting ? <X size={16} /> : <ListChecks size={16} />}
          </button>
          <button type="button" className="new-session-button" onClick={() => create.mutate()} disabled={create.isPending}>
            <Plus size={15} /><span>{create.isPending ? "正在新建" : "新会话"}</span>
          </button>
        </div>
      </div>
      {selecting && (
        <div className="session-bulk-bar">
          <button type="button" onClick={toggleAll}>{selected.size === manageableCount && manageableCount > 0 ? <CheckSquare2 size={14} /> : <Square size={14} />}全选</button>
          <span>已选择 {selected.size} 项</span>
          <button type="button" className={confirming ? "danger confirming" : "danger"} onClick={requestBulkDelete} disabled={selected.size === 0 || removeMany.isPending}>
            <Trash2 size={14} />{removeMany.isPending ? "正在删除" : confirming ? `确认删除 ${selected.size} 项` : "删除"}
          </button>
        </div>
      )}
      <div className="session-list">
        {sessions.isLoading && <div className="sidebar-state"><RefreshCw size={15} className="spin" /> 读取会话</div>}
        {sessions.data?.map((session) => {
          const checked = selected.has(session.id);
          return (
            <div className={`${session.active ? "session-row active" : "session-row"}${checked ? " selected" : ""}`} key={session.id}>
              {selecting && (
                <button type="button" className="session-check" onClick={() => toggleSelected(session.id)} disabled={session.id === "default"} aria-label={`选择 ${session.title}`}>
                  {checked ? <CheckSquare2 size={15} /> : <Square size={15} />}
                </button>
              )}
              {!selecting && renaming === session.id ? (
                <div className="session-rename">
                  <MessageSquare size={14} />
                  <input
                    autoFocus
                    value={renameDraft}
                    disabled={rename.isPending}
                    onChange={(event) => setRenameDraft(event.target.value)}
                    onKeyDown={(event) => {
                      // 1. 回车提交重命名
                      if (event.key === "Enter") submitRename();
                      // 2. Esc 取消编辑
                      if (event.key === "Escape") setRenaming(null);
                    }}
                    onBlur={() => setRenaming(null)}
                    aria-label={`重命名 ${session.title}`}
                  />
                </div>
              ) : (
                <button type="button" className="session-main" onClick={() => {
                  if (selecting) {
                    if (session.id !== "default") toggleSelected(session.id);
                    return;
                  }
                  if (!session.active) switchSession.mutate(session.id);
                  onNavigate?.();
                }}>
                  <MessageSquare size={14} />
                  <span><strong>{session.title}</strong><small>{new Date(session.updated_at).toLocaleString()}</small></span>
                </button>
              )}
              {!selecting && renaming !== session.id && <button type="button" className="session-more" aria-label={`管理 ${session.title}`} onClick={() => setMenu((value) => value === session.id ? null : session.id)}><MoreHorizontal size={15} /></button>}
              {!selecting && menu === session.id && (
                <div className="session-menu" ref={menuRef}>
                  <button type="button" onClick={() => startRename(session.id, session.title)}><Pencil size={14} /> 重命名</button>
                  <button type="button" className="danger" disabled={session.id === "default"} onClick={() => { remove.mutate(session.id); setMenu(null); }}><Trash2 size={14} /> 删除</button>
                </div>
              )}
            </div>
          );
        })}
      </div>
      {error && <p className="sidebar-error">{error.message}</p>}
      <div className="sidebar-footer">
        <NavLink to="/settings" onClick={onNavigate} className={({ isActive }) => isActive ? "sidebar-settings-link active" : "sidebar-settings-link"}>
          <Settings size={15} strokeWidth={1.8} /><span>配置</span>
        </NavLink>
      </div>
    </div>
  );
}
