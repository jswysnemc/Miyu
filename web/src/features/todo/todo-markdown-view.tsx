import { useQuery } from "@tanstack/react-query";
import { Ban, CheckCircle2, ChevronDown, Circle, CircleDot } from "lucide-react";
import { useState } from "react";
import { api } from "../../api/client";
import type { TodoStatus } from "../../api/contracts";
import "./todo-markdown.css";

const statusIcons = { pending:Circle,in_progress:CircleDot,completed:CheckCircle2,cancelled:Ban } satisfies Record<TodoStatus,typeof Circle>;

/** 渲染 Agent 管理的只读 TODO 进度。 */
export function TodoMarkdownView({ sessionId,compact=false }: { sessionId?:string;compact?:boolean }) {
  const [open,setOpen]=useState(false);
  const query=useQuery({ queryKey:["todos",sessionId],queryFn:api.todos.list,enabled:Boolean(sessionId),refetchInterval:2000 });
  if(!sessionId || !query.data?.length) return null;
  const items=query.data; const completed=items.filter((item)=>item.status==="completed").length;
  return <section className={`todo-markdown-view${compact ? " compact" : ""}`}>
    <button type="button" className="todo-markdown-trigger" onClick={()=>setOpen((value)=>!value)} aria-expanded={open}>
      <strong>TODO {completed}/{items.length} 条目</strong><ChevronDown size={15} className={open ? "open" : ""}/>
    </button>
    {open && <ul className="todo-markdown-list">{items.map((item)=>{const Icon=statusIcons[item.status];return <li key={item.id} className={`todo-markdown-item is-${item.status}`}><Icon size={15}/><span>{item.text}</span></li>;})}</ul>}
    {query.error && <div className="run-error">{query.error.message}</div>}
  </section>;
}
