import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { MessageSquare, MoreHorizontal, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useState } from "react";
import { api } from "../../api/client";
import "./session-sidebar.css";

export function SessionSidebar() {
  const queryClient = useQueryClient();
  const [menu, setMenu] = useState<string | null>(null);
  const sessions = useQuery({ queryKey: ["sessions"], queryFn: api.sessions.list });
  const refresh = async () => {
    await queryClient.invalidateQueries({ queryKey: ["sessions"] });
    await queryClient.invalidateQueries({ queryKey: ["messages"] });
  };
  const create = useMutation({ mutationFn: () => api.sessions.create(), onSuccess: refresh });
  const switchSession = useMutation({ mutationFn: api.sessions.switch, onSuccess: refresh });
  const remove = useMutation({ mutationFn: api.sessions.remove, onSuccess: refresh });

  return (
    <div className="session-sidebar">
      <div className="sidebar-heading">
        <div>
          <span className="eyebrow">Sessions</span>
          <h2>会话</h2>
        </div>
        <button type="button" className="icon-button" aria-label="新建会话" onClick={() => create.mutate()} disabled={create.isPending}>
          <Plus size={17} />
        </button>
      </div>
      <div className="session-list">
        {sessions.isLoading && <div className="sidebar-state"><RefreshCw size={15} className="spin" /> 读取会话</div>}
        {sessions.data?.map((session) => (
          <div className={session.active ? "session-row active" : "session-row"} key={session.id}>
            <button type="button" className="session-main" onClick={() => !session.active && switchSession.mutate(session.id)}>
              <MessageSquare size={14} />
              <span>
                <strong>{session.title}</strong>
                <small>{new Date(session.updated_at).toLocaleString()}</small>
              </span>
            </button>
            <button type="button" className="session-more" aria-label={`管理 ${session.title}`} onClick={() => setMenu((value) => value === session.id ? null : session.id)}>
              <MoreHorizontal size={15} />
            </button>
            {menu === session.id && (
              <div className="session-menu">
                <button type="button" disabled={session.id === "default"} onClick={() => { remove.mutate(session.id); setMenu(null); }}>
                  <Trash2 size={14} /> 删除
                </button>
              </div>
            )}
          </div>
        ))}
      </div>
      {(sessions.error || switchSession.error || remove.error) && (
        <p className="sidebar-error">{(sessions.error ?? switchSession.error ?? remove.error)?.message}</p>
      )}
    </div>
  );
}
