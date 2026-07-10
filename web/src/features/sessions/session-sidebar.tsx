import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckSquare2, ListChecks, MessageSquare, MoreHorizontal, Plus, RefreshCw, Settings, Square, Trash2, X } from "lucide-react";
import { useState } from "react";
import { NavLink } from "react-router-dom";
import { api } from "../../api/client";
import "./session-sidebar.css";

/**
 * 渲染会话列表、新建入口和批量管理模式。
 *
 * @returns 会话侧栏
 */
export function SessionSidebar() {
  const queryClient = useQueryClient();
  const [menu, setMenu] = useState<string | null>(null);
  const [selecting, setSelecting] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState(false);
  const sessions = useQuery({ queryKey: ["sessions"], queryFn: api.sessions.list });

  /** 刷新会话列表和全部消息缓存。 */
  const refresh = async () => {
    await queryClient.invalidateQueries({ queryKey: ["sessions"] });
    await queryClient.invalidateQueries({ queryKey: ["messages"] });
    await queryClient.invalidateQueries({ queryKey: ["timeline"] });
  };

  const create = useMutation({ mutationFn: () => api.sessions.create(), onSuccess: refresh });
  const switchSession = useMutation({ mutationFn: api.sessions.switch, onSuccess: refresh });
  const remove = useMutation({ mutationFn: api.sessions.remove, onSuccess: refresh });
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

  const manageableCount = sessions.data?.filter((session) => session.id !== "default").length ?? 0;
  const error = sessions.error ?? switchSession.error ?? remove.error ?? removeMany.error;
  return (
    <div className="session-sidebar">
      <div className="sidebar-heading">
        <div><span className="eyebrow">Sessions</span><h2>会话</h2></div>
        <div className="sidebar-heading-actions">
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
              <button type="button" className="session-main" onClick={() => selecting ? session.id !== "default" && toggleSelected(session.id) : !session.active && switchSession.mutate(session.id)}>
                <MessageSquare size={14} />
                <span><strong>{session.title}</strong><small>{new Date(session.updated_at).toLocaleString()}</small></span>
              </button>
              {!selecting && <button type="button" className="session-more" aria-label={`管理 ${session.title}`} onClick={() => setMenu((value) => value === session.id ? null : session.id)}><MoreHorizontal size={15} /></button>}
              {!selecting && menu === session.id && (
                <div className="session-menu"><button type="button" disabled={session.id === "default"} onClick={() => { remove.mutate(session.id); setMenu(null); }}><Trash2 size={14} /> 删除</button></div>
              )}
            </div>
          );
        })}
      </div>
      {error && <p className="sidebar-error">{error.message}</p>}
      <div className="sidebar-footer">
        <NavLink to="/settings" className={({ isActive }) => isActive ? "sidebar-settings-link active" : "sidebar-settings-link"}>
          <Settings size={15} strokeWidth={1.8} /><span>配置</span>
        </NavLink>
      </div>
    </div>
  );
}
