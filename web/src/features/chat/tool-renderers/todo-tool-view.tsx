import { Ban, CheckCircle2, ChevronDown, Circle, CircleDot, ListChecks } from "lucide-react";
import { useState } from "react";
import type { TodoStatus } from "../../../api/contracts";
import { parseTodoTool, statusLabel, todoToolHeadline } from "./todo-tool-data";
import "./todo-tool-view.css";

const statusIcons = { pending: Circle, in_progress: CircleDot, completed: CheckCircle2, cancelled: Ban } satisfies Record<TodoStatus, typeof Circle>;

type TodoToolItem = { id: string; text: string; status: TodoStatus };

/**
 * 渲染消息流中的 todo 工具调用卡片。
 *
 * 折叠态展示一句摘要(创建/更新/删除了什么),展开态展示该次调用返回的清单快照,
 * 避免直接暴露原始 JSON。
 *
 * @param props todo 工具调用的参数与输出
 * @returns todo 工具卡片
 */
export function TodoToolView({ argumentsText, output }: { argumentsText: string; output: string }) {
  const [expanded, setExpanded] = useState(false);
  const summary = parseTodoTool(argumentsText, output);
  const headline = todoToolHeadline(summary);
  const items = parseItems(output);
  const canExpand = items.length > 0;
  return (
    <div className={`todo-tool-view is-${summary.action}`}>
      <button
        type="button"
        className="todo-tool-head"
        onClick={() => canExpand && setExpanded((value) => !value)}
        aria-expanded={canExpand ? expanded : undefined}
        disabled={!canExpand}
      >
        <span className="todo-tool-icon"><ListChecks size={14} /></span>
        <span className="todo-tool-headline">{headline}</span>
        {summary.status && summary.action === "update" && <span className={`todo-tool-tag is-${summary.status}`}>{statusLabel(summary.status)}</span>}
        {canExpand && <ChevronDown size={14} className={expanded ? "open" : ""} />}
      </button>
      {expanded && canExpand && (
        <ul className="todo-tool-list">
          {items.map((item) => {
            const Icon = statusIcons[item.status] ?? Circle;
            return (
              <li key={item.id} className={`todo-tool-item is-${item.status}`}>
                <Icon size={14} /><span>{item.text}</span>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

/**
 * 从 todo 工具输出中解析清单条目。
 *
 * @param output todo 工具输出 JSON
 * @returns 清单条目,无清单时为空数组
 */
function parseItems(output: string): TodoToolItem[] {
  try {
    const value = JSON.parse(output) as { items?: unknown };
    if (!Array.isArray(value.items)) return [];
    return value.items.filter(isTodoItem);
  } catch {
    return [];
  }
}

/** 判断值是否为合法的 todo 条目。 */
function isTodoItem(value: unknown): value is TodoToolItem {
  return typeof value === "object" && value !== null
    && typeof (value as TodoToolItem).id === "string"
    && typeof (value as TodoToolItem).text === "string"
    && typeof (value as TodoToolItem).status === "string";
}
