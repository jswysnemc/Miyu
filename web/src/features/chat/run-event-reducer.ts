import type { WebEvent } from "../../api/contracts";

export type ToolLifecycle = {
  id: string;
  name: string;
  argumentsPreview: string;
  arguments: string;
  progress: string;
  output: string;
  status: "preparing" | "running" | "completed" | "failed";
};

export type LiveMessagePart =
  | { id: string; type: "reasoning"; source: string; startedAt: string; endedAt?: string }
  | { id: string; type: "text"; source: string }
  | { id: string; type: "tool"; tool: ToolLifecycle };

export type LiveRunState = {
  runId: string | null;
  status: "idle" | "waiting_response" | "thinking" | "working";
  userInput: string;
  imageUrls: string[];
  content: string;
  reasoning: string;
  tools: ToolLifecycle[];
  parts: LiveMessagePart[];
  error: string | null;
  completed: boolean;
};

export type RunAction =
  | { type: "start"; runId: string; userInput: string; imageUrls?: string[] }
  | { type: "event"; event: WebEvent }
  | { type: "reset" };

export const initialRunState: LiveRunState = {
  runId: null,
  status: "idle",
  userInput: "",
  imageUrls: [],
  content: "",
  reasoning: "",
  tools: [],
  parts: [],
  error: null,
  completed: false
};

/** 将后端事件归并为单轮聊天与工具生命周期状态。 */
export function runEventReducer(state: LiveRunState, action: RunAction): LiveRunState {
  if (action.type === "reset") return initialRunState;
  if (action.type === "start") {
    return { ...initialRunState, runId: action.runId, userInput: action.userInput, imageUrls: action.imageUrls ?? [], status: "waiting_response" };
  }
  const { event } = action;
  const payload = event.payload;
  switch (event.type) {
    case "status.changed":
      return { ...state, status: String(payload.status) as LiveRunState["status"] };
    case "message.content.delta":
      return appendTextPart(closeActiveReasoning(state, event.timestamp), event.sequence, String(payload.text ?? ""));
    case "message.reasoning.delta":
      return appendReasoningPart(state, event.sequence, event.timestamp, String(payload.text ?? ""));
    case "tool.call.preparing":
      return upsertTool(closeActiveReasoning(state, event.timestamp), String(payload.tool_id), {
        name: String(payload.name ?? "tool"),
        argumentsPreview: String(payload.arguments_preview ?? ""),
        status: "preparing"
      });
    case "tool.call.started":
      return upsertTool(closeActiveReasoning(state, event.timestamp), String(payload.tool_id), {
        name: String(payload.name ?? "tool"),
        arguments: String(payload.arguments ?? ""),
        argumentsPreview: String(payload.arguments ?? ""),
        status: "running"
      });
    case "tool.progress":
      return upsertTool(closeActiveReasoning(state, event.timestamp), String(payload.tool_id), {
        name: String(payload.name ?? "tool"),
        progress: String(payload.message ?? ""),
        status: "running"
      });
    case "tool.result":
      return upsertTool(closeActiveReasoning(state, event.timestamp), String(payload.tool_id), {
        name: String(payload.name ?? "tool"),
        output: String(payload.output ?? ""),
        status: payload.ok === false ? "failed" : "completed"
      });
    case "run.failed":
      return { ...closeActiveReasoning(state, event.timestamp), error: String(payload.message ?? "运行失败"), status: "idle", completed: true };
    case "run.interrupted":
      return { ...closeActiveReasoning(state, event.timestamp), error: "运行已中断", status: "idle", completed: true };
    case "run.completed":
      return { ...closeActiveReasoning(state, event.timestamp), status: "idle", completed: true };
    default:
      return state;
  }
}

/**
 * 按工具 ID 创建或更新工具卡片。
 *
 * @param state 当前运行状态
 * @param id 工具 ID
 * @param patch 增量字段
 * @returns 更新后的运行状态
 */
function upsertTool(state: LiveRunState, id: string, patch: Partial<ToolLifecycle>): LiveRunState {
  const index = state.tools.findIndex((tool) => tool.id === id);
  const base: ToolLifecycle = {
    id,
    name: "tool",
    argumentsPreview: "",
    arguments: "",
    progress: "",
    output: "",
    status: "preparing"
  };
  if (index === -1) {
    const tool = { ...base, ...patch };
    return { ...state, tools: [...state.tools, tool], parts: [...state.parts, { id: `tool-${id}`, type: "tool", tool }] };
  }
  // 1. 后端 ID 配对异常时同一 ID 可能带来不同工具名，此时另建卡片避免覆盖已有视图
  const existing = state.tools[index];
  if (patch.name && existing.name !== "tool" && patch.name !== existing.name) {
    const forkedId = `${id}-${patch.name}`;
    return upsertTool(state, forkedId, patch);
  }
  const tools = state.tools.map((tool, toolIndex) => toolIndex === index ? { ...tool, ...patch } : tool);
  return {
    ...state,
    tools,
    parts: state.parts.map((part) => part.type === "tool" && part.tool.id === id ? { ...part, tool: tools[index] } : part)
  };
}

/**
 * 将正文增量追加到当前连续正文部件。
 *
 * @param state 当前运行状态
 * @param sequence 事件序号
 * @param text 正文增量
 * @returns 更新后的运行状态
 */
function appendTextPart(state: LiveRunState, sequence: number, text: string): LiveRunState {
  const last = state.parts.at(-1);
  const parts = last?.type === "text"
    ? state.parts.map((part, index) => index === state.parts.length - 1 && part.type === "text" ? { ...part, source: part.source + text } : part)
    : [...state.parts, { id: `text-${sequence}`, type: "text" as const, source: text }];
  return { ...state, content: state.content + text, parts };
}

/**
 * 将思考增量追加到当前连续思考部件。
 *
 * @param state 当前运行状态
 * @param sequence 事件序号
 * @param timestamp 事件时间
 * @param text 思考增量
 * @returns 更新后的运行状态
 */
function appendReasoningPart(state: LiveRunState, sequence: number, timestamp: string, text: string): LiveRunState {
  const last = state.parts.at(-1);
  const parts = last?.type === "reasoning" && !last.endedAt
    ? state.parts.map((part, index) => index === state.parts.length - 1 && part.type === "reasoning" ? { ...part, source: part.source + text } : part)
    : [...state.parts, { id: `reasoning-${sequence}`, type: "reasoning" as const, source: text, startedAt: timestamp }];
  return { ...state, reasoning: state.reasoning + text, parts };
}

/**
 * 在正文或工具事件开始前结束当前思考部件。
 *
 * @param state 当前运行状态
 * @param timestamp 结束时间
 * @returns 更新后的运行状态
 */
function closeActiveReasoning(state: LiveRunState, timestamp: string): LiveRunState {
  const last = state.parts.at(-1);
  if (last?.type !== "reasoning" || last.endedAt) return state;
  return {
    ...state,
    parts: state.parts.map((part, index) => index === state.parts.length - 1 && part.type === "reasoning" ? { ...part, endedAt: timestamp } : part)
  };
}
