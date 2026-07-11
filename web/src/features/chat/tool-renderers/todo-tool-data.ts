export type TodoToolAction = "list" | "add" | "update" | "remove" | "unknown";

export type TodoToolSummary = {
  action: TodoToolAction;
  text: string;
  status: string;
  itemCount: number | null;
};

/**
 * 解析 todo 工具调用的参数与输出,提炼一条简要摘要。
 *
 * @param argumentsText todo 工具调用参数 JSON
 * @param output todo 工具调用输出 JSON
 * @returns todo 调用摘要
 */
export function parseTodoTool(argumentsText: string, output: string): TodoToolSummary {
  const args = safeParse(argumentsText);
  const result = safeParse(output);
  const action = normalizeAction(typeof args.action === "string" ? args.action : "");
  const items = Array.isArray(result.items) ? result.items : null;
  const item = isRecord(result.item) ? result.item : null;
  return {
    action,
    text: readText(args, item),
    status: readStatus(args, item),
    itemCount: items ? items.length : null
  };
}

/**
 * 生成 todo 调用的中文摘要句。
 *
 * @param summary todo 调用摘要
 * @returns 摘要文本
 */
export function todoToolHeadline(summary: TodoToolSummary): string {
  switch (summary.action) {
    case "add":
      return summary.text ? `创建任务：${summary.text}` : "创建了一个任务";
    case "update":
      return summary.text
        ? `更新任务：${summary.text}`
        : summary.status
          ? `更新任务状态为${statusLabel(summary.status)}`
          : "更新了任务";
    case "remove":
      return summary.text ? `删除任务：${summary.text}` : "删除了一个任务";
    case "list":
      return summary.itemCount !== null ? `查看计划清单（${summary.itemCount} 项）` : "查看计划清单";
    default:
      return "更新计划清单";
  }
}

/**
 * 返回 todo 状态的中文名称。
 *
 * @param status 状态标识
 * @returns 状态名称
 */
export function statusLabel(status: string): string {
  return ({ pending: "待处理", in_progress: "进行中", completed: "已完成", cancelled: "已取消" } as Record<string, string>)[status] ?? status;
}

/** 解析 JSON,失败时返回空对象。 */
function safeParse(text: string): Record<string, unknown> {
  try {
    const value = JSON.parse(text);
    return isRecord(value) ? value : {};
  } catch {
    return {};
  }
}

/** 判断值是否为普通对象。 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** 归一化 action 字段。 */
function normalizeAction(action: string): TodoToolAction {
  return (["list", "add", "update", "remove"] as const).find((item) => item === action) ?? "unknown";
}

/** 优先从参数、其次从结果读取任务文本。 */
function readText(args: Record<string, unknown>, item: Record<string, unknown> | null): string {
  if (typeof args.text === "string" && args.text.trim()) return args.text.trim();
  if (item && typeof item.text === "string") return item.text;
  return "";
}

/** 优先从参数、其次从结果读取任务状态。 */
function readStatus(args: Record<string, unknown>, item: Record<string, unknown> | null): string {
  if (typeof args.status === "string" && args.status.trim()) return args.status.trim();
  if (item && typeof item.status === "string") return item.status;
  return "";
}
