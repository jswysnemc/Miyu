import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Ban, RefreshCw } from "lucide-react";
import { api } from "../../api/client";
import { useState } from "react";
import { MarkdownRenderer } from "../chat/markdown-renderer";
import "./subagents.css";

/** 渲染进程内子智能体状态与控制。 */
export function SubagentPanel() {
  const queryClient=useQueryClient();
  const [selectedId,setSelectedId]=useState<string | null>(null);
  const query=useQuery({ queryKey:["subagents"], queryFn:api.subagents.list, refetchInterval:2000 });
  const cancel=useMutation({ mutationFn:api.subagents.cancel, onSuccess:()=>void queryClient.invalidateQueries({queryKey:["subagents"]}) });
  const selected=query.data?.find((subagent)=>subagent.id===selectedId);
  if(selected) return <section className="subagent-panel subagent-detail"><header><button type="button" onClick={()=>setSelectedId(null)}>返回概览</button><strong>{selected.description}</strong></header><MarkdownRenderer source={selected.result || selected.error || "子智能体正在运行，等待结果。"}/></section>;
  return <section className="subagent-panel">
    <header><div><strong>子智能体</strong><span>{query.data?.length ?? 0} 个</span></div><button type="button" onClick={()=>void query.refetch()} aria-label="刷新子智能体"><RefreshCw size={14}/></button></header>
    <div className="subagent-list">{query.data?.map((subagent)=><article key={subagent.id} onClick={()=>setSelectedId(subagent.id)}>
      <div className="subagent-heading"><span className={`subagent-status ${subagent.status}`}>{subagent.status}</span><strong>{subagent.description}</strong></div>
      <dl><div><dt>类型</dt><dd>{subagent.subagent_type}</dd></div><div><dt>最大步骤</dt><dd>{subagent.max_steps}</dd></div></dl>
      {subagent.result && <pre>{subagent.result}</pre>}{subagent.error && <p className="subagent-error">{subagent.error}</p>}
      {subagent.status==="running" && <button type="button" className="subagent-cancel" onClick={(event)=>{event.stopPropagation();cancel.mutate(subagent.id);}}><Ban size={13}/>取消</button>}
    </article>)}</div>
    {!query.isLoading && query.data?.length===0 && <p className="management-empty">没有子智能体</p>}
    {(query.error || cancel.error) && <div className="pane-error">{(query.error ?? cancel.error)?.message}</div>}
  </section>;
}
