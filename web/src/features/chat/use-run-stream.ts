import { useEffect, useReducer } from "react";
import type { RunModelSelection, ThinkingLevel, WebEvent } from "../../api/contracts";
import { api } from "../../api/client";
import { initialRunState, runEventReducer } from "./run-event-reducer";

const EVENT_TYPES = [
  "run.started",
  "status.changed",
  "message.content.delta",
  "message.reasoning.delta",
  "tool.call.preparing",
  "tool.call.started",
  "tool.progress",
  "tool.result",
  "content.flushed",
  "compaction.started",
  "compaction.finished",
  "loaded_tools.changed",
  "session.summary",
  "run.completed",
  "run.interrupted",
  "run.failed"
] as const;

/** 管理一轮 Agent 提交和 SSE 事件连接。 */
export function useRunStream(onSettled: () => void) {
  const [state, dispatch] = useReducer(runEventReducer, initialRunState);

  useEffect(() => {
    if (!state.runId || state.completed) return;
    const source = new EventSource(`/api/runs/${state.runId}/events`);
    const handle = (message: MessageEvent<string>) => {
      const event = JSON.parse(message.data) as WebEvent;
      dispatch({ type: "event", event });
      if (["run.completed", "run.interrupted", "run.failed"].includes(event.type)) {
        source.close();
        onSettled();
      }
    };
    for (const type of EVENT_TYPES) source.addEventListener(type, handle as EventListener);
    source.onerror = () => {
      // EventSource 会按 Last-Event-ID 自动重连，后端事件日志负责补发
    };
    return () => source.close();
  }, [state.runId, state.completed, onSettled]);

  /**
   * 启动一轮 Agent 运行。
   *
   * @param sessionId 会话标识
   * @param input 用户输入
   * @param mode 运行模式
   * @param selection 本轮供应商和模型
   * @param imageUrls 图片 data URL 列表
   * @param thinkingLevel 本轮思考等级
   */
  const start = async (
    sessionId: string,
    input: string,
    mode: "plan" | "yolo",
    selection?: RunModelSelection,
    imageUrls?: string[],
    thinkingLevel?: ThinkingLevel
  ) => {
    const run = await api.runs.start(sessionId, input, mode, selection, imageUrls, thinkingLevel);
    dispatch({ type: "start", runId: run.run_id, userInput: input, imageUrls });
  };

  const stop = async () => {
    if (state.runId) await api.runs.stop(state.runId);
  };

  return { state, start, stop, reset: () => dispatch({ type: "reset" }) };
}
