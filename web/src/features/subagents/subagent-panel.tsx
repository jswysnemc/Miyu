import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, RefreshCw } from "lucide-react";
import { useState } from "react";
import { api } from "../../api/client";
import { MarkdownRenderer } from "../chat/markdown-renderer";
import { SubagentCard } from "./subagent-card";
import { SubagentProgress } from "./subagent-progress";
import { SubagentStatusBadge } from "./subagent-status-badge";
import "./subagents.css";

/** 渲染进程内子智能体状态与控制。 */
export function SubagentPanel() {
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const query = useQuery({ queryKey: ["subagents"], queryFn: api.subagents.list, refetchInterval: 2000 });
  const cancel = useMutation({
    mutationFn: api.subagents.cancel,
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["subagents"] })
  });
  const selected = query.data?.find((subagent) => subagent.id === selectedId);
  const running = query.data?.filter((subagent) => subagent.status === "running").length ?? 0;

  if (selected) {
    return (
      <section className="subagent-panel subagent-detail">
        <header>
          <button type="button" onClick={() => setSelectedId(null)}><ArrowLeft size={13} />返回概览</button>
          <SubagentStatusBadge status={selected.status} />
          <strong>{selected.description}</strong>
        </header>
        <SubagentProgress subagent={selected} />
        <MarkdownRenderer source={selected.result || selected.error || "子智能体正在运行，等待结果。"} />
      </section>
    );
  }

  return (
    <section className="subagent-panel">
      <header>
        <div><strong>子智能体</strong><span>{running > 0 ? `${running} 个运行中` : `${query.data?.length ?? 0} 个`}</span></div>
        <button type="button" onClick={() => void query.refetch()} aria-label="刷新子智能体"><RefreshCw size={14} /></button>
      </header>
      <div className="subagent-list">
        {query.data?.map((subagent) => (
          <SubagentCard
            key={subagent.id}
            subagent={subagent}
            onSelect={() => setSelectedId(subagent.id)}
            onCancel={() => cancel.mutate(subagent.id)}
          />
        ))}
      </div>
      {!query.isLoading && query.data?.length === 0 && <p className="management-empty">没有子智能体</p>}
      {(query.error || cancel.error) && <div className="pane-error">{(query.error ?? cancel.error)?.message}</div>}
    </section>
  );
}
