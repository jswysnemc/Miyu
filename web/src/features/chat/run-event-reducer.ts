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

export type LiveRunState = {
  runId: string | null;
  status: "idle" | "waiting_response" | "thinking" | "working";
  userInput: string;
  content: string;
  reasoning: string;
  tools: ToolLifecycle[];
  error: string | null;
  completed: boolean;
};

export type RunAction =
  | { type: "start"; runId: string; userInput: string }
  | { type: "event"; event: WebEvent }
  | { type: "reset" };

export const initialRunState: LiveRunState = {
  runId: null,
  status: "idle",
  userInput: "",
  content: "",
  reasoning: "",
  tools: [],
  error: null,
  completed: false
};

/** 将后端事件归并为单轮聊天与工具生命周期状态。 */
export function runEventReducer(state: LiveRunState, action: RunAction): LiveRunState {
  if (action.type === "reset") return initialRunState;
  if (action.type === "start") {
    return { ...initialRunState, runId: action.runId, userInput: action.userInput, status: "waiting_response" };
  }
  const { event } = action;
  const payload = event.payload;
  switch (event.type) {
    case "status.changed":
      return { ...state, status: String(payload.status) as LiveRunState["status"] };
    case "message.content.delta":
      return { ...state, content: state.content + String(payload.text ?? "") };
    case "message.reasoning.delta":
      return { ...state, reasoning: state.reasoning + String(payload.text ?? "") };
    case "tool.call.preparing":
      return upsertTool(state, String(payload.tool_id), {
        name: String(payload.name ?? "tool"),
        argumentsPreview: String(payload.arguments_preview ?? ""),
        status: "preparing"
      });
    case "tool.call.started":
      return upsertTool(state, String(payload.tool_id), {
        name: String(payload.name ?? "tool"),
        arguments: String(payload.arguments ?? ""),
        argumentsPreview: String(payload.arguments ?? ""),
        status: "running"
      });
    case "tool.progress":
      return upsertTool(state, String(payload.tool_id), {
        name: String(payload.name ?? "tool"),
        progress: String(payload.message ?? ""),
        status: "running"
      });
    case "tool.result":
      return upsertTool(state, String(payload.tool_id), {
        name: String(payload.name ?? "tool"),
        output: String(payload.output ?? ""),
        status: payload.ok === false ? "failed" : "completed"
      });
    case "run.failed":
      return { ...state, error: String(payload.message ?? "运行失败"), status: "idle", completed: true };
    case "run.interrupted":
      return { ...state, error: "运行已中断", status: "idle", completed: true };
    case "run.completed":
      return { ...state, status: "idle", completed: true };
    default:
      return state;
  }
}

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
  if (index === -1) return { ...state, tools: [...state.tools, { ...base, ...patch }] };
  return {
    ...state,
    tools: state.tools.map((tool, toolIndex) => toolIndex === index ? { ...tool, ...patch } : tool)
  };
}
