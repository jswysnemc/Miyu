import { useQuery } from "@tanstack/react-query";
import { Ban, CheckCircle2, ChevronDown, Circle, CircleDot, ListChecks } from "lucide-react";
import { useRef, useState } from "react";
import { api } from "../../api/client";
import type { TodoStatus } from "../../api/contracts";
import { useOutsidePointerDown } from "../../shared/hooks/use-outside-pointer-down";
import { summarizeTodos } from "./todo-summary";
import "./todo-markdown.css";

const statusIcons = { pending: Circle, in_progress: CircleDot, completed: CheckCircle2, cancelled: Ban } satisfies Record<TodoStatus, typeof Circle>;

/** 渲染 Agent 管理的只读 TODO 进度。 */
export function TodoMarkdownView({ sessionId, compact = false }: { sessionId?: string; compact?: boolean }) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLElement>(null);
  // 1. 紧凑弹窗模式下点击外部关闭清单
  useOutsidePointerDown(rootRef, () => setOpen(false), compact && open);
  const query = useQuery({ queryKey: ["todos", sessionId], queryFn: api.todos.list, enabled: Boolean(sessionId), refetchInterval: 2000 });
  if (!sessionId || !query.data?.length) return null;
  const items = query.data;
  const summary = summarizeTodos(items);
  const percent = Math.round(summary.ratio * 100);
  return (
    <section ref={rootRef} className={`todo-markdown-view${compact ? " compact" : ""}`}>
      <button type="button" className="todo-markdown-trigger" onClick={() => setOpen((value) => !value)} aria-expanded={open}>
        <span className="todo-trigger-icon"><ListChecks size={14} /></span>
        <span className="todo-trigger-body">
          <span className="todo-trigger-line">
            <strong>{summary.allDone ? "计划已完成" : summary.activeText || "计划"}</strong>
            <span className="todo-trigger-count">{summary.completed}/{summary.total}</span>
          </span>
          <span className="todo-trigger-track" aria-hidden><span className="todo-trigger-fill" style={{ width: `${percent}%` }} /></span>
        </span>
        <ChevronDown size={15} className={open ? "open" : ""} />
      </button>
      {open && (
        <ul className="todo-markdown-list">
          {items.map((item) => {
            const Icon = statusIcons[item.status];
            return (
              <li key={item.id} className={`todo-markdown-item is-${item.status}`}>
                <Icon size={15} />
                <span>{item.text}</span>
              </li>
            );
          })}
        </ul>
      )}
      {query.error && <div className="run-error">{query.error.message}</div>}
    </section>
  );
}
