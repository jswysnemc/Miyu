import { useEffect, useMemo, useReducer, useRef } from "react";
import type { RunInfo, RunModelSelection, ThinkingLevel, WebEvent } from "../../api/contracts";
import { api } from "../../api/client";
import { initialRunState, runEventReducer, type LiveRunState } from "./run-event-reducer";

const EVENT_TYPES = [
  "run.queued",
  "run.dequeued",
  "run.started",
  "status.changed",
  "message.content.delta",
  "message.reasoning.delta",
  "tool.call.preparing",
  "tool.call.started",
  "tool.progress",
  "tool.result",
  "workspace.changed",
  "content.flushed",
  "compaction.started",
  "compaction.finished",
  "loaded_tools.changed",
  "session.summary",
  "run.completed",
  "run.interrupted",
  "run.failed"
] as const;

type SessionRunsState = { runs: LiveRunState[] };

type SessionRunsAction =
  | { type: "attach"; runs: RunInfo[]; sessionId: string }
  | { type: "start"; run: RunInfo; sessionId: string; userInput: string; imageUrls?: string[] }
  | { type: "event"; event: WebEvent }
  | { type: "reset" };

const initialSessionRunsState: SessionRunsState = { runs: [] };

/** 将运行事件归并到会话内对应的实时消息。 */
function sessionRunsReducer(state: SessionRunsState, action: SessionRunsAction): SessionRunsState {
  if (action.type === "reset") return initialSessionRunsState;
  if (action.type === "attach") {
    const known = new Set(state.runs.map((run) => run.runId));
    const attached = action.runs
      .filter((run) => !known.has(run.run_id))
      .map((run) => ({
        ...runEventReducer(initialRunState, {
          type: "attach",
          runId: run.run_id,
          sessionId: action.sessionId,
          userInput: run.input ?? "",
          imageUrls: run.image_urls
        }),
        status: run.status === "queued" ? "queued" as const : "waiting_response" as const
      }));
    return { runs: [...state.runs, ...attached] };
  }
  if (action.type === "start") {
    const next = runEventReducer(initialRunState, {
      type: "start",
      runId: action.run.run_id,
      sessionId: action.sessionId,
      userInput: action.userInput,
      imageUrls: action.imageUrls
    });
    return {
      runs: [...state.runs, {
        ...next,
        status: action.run.status === "queued" ? "queued" : next.status
      }]
    };
  }
  return {
    runs: state.runs.map((run) => run.runId === action.event.run_id
      ? runEventReducer(run, { type: "event", event: action.event })
      : run)
  };
}

/** 管理一个会话中的活动和排队 Agent 运行。 */
export function useRunStream(workspaceId: string | undefined, sessionId: string | undefined, onSettled: () => void, onWorkspaceChanged?: () => void) {
  const [state, dispatch] = useReducer(sessionRunsReducer, initialSessionRunsState);
  const sourcesRef = useRef(new Map<string, EventSource>());

  useEffect(() => {
    if (!workspaceId || !sessionId) return;
    let cancelled = false;
    void api.runs.active().then(({ runs }) => {
      if (cancelled) return;
      dispatch({
        type: "attach",
        sessionId,
        runs: runs.filter((run) => run.workspace_id === workspaceId && run.session_id === sessionId)
      });
    });
    return () => { cancelled = true; };
  }, [workspaceId, sessionId]);

  const openRunIds = useMemo(
    () => state.runs.filter((run) => run.runId && !run.completed).map((run) => run.runId!),
    [state.runs]
  );
  const openRunKey = openRunIds.join(",");

  useEffect(() => {
    const desired = new Set(openRunIds);
    for (const [runId, source] of sourcesRef.current) {
      if (desired.has(runId)) continue;
      source.close();
      sourcesRef.current.delete(runId);
    }
    for (const runId of openRunIds) {
      if (sourcesRef.current.has(runId)) continue;
      const source = new EventSource(`/api/runs/${runId}/events`);
      const handle = (message: MessageEvent<string>) => {
        const event = JSON.parse(message.data) as WebEvent;
        dispatch({ type: "event", event });
        if (event.type === "workspace.changed") onWorkspaceChanged?.();
        if (["run.completed", "run.interrupted", "run.failed"].includes(event.type)) {
          source.close();
          sourcesRef.current.delete(runId);
          onSettled();
        }
      };
      for (const type of EVENT_TYPES) source.addEventListener(type, handle as EventListener);
      sourcesRef.current.set(runId, source);
    }
  }, [openRunKey, onSettled, onWorkspaceChanged]);

  useEffect(() => () => {
    for (const source of sourcesRef.current.values()) source.close();
    sourcesRef.current.clear();
  }, []);

  /**
   * 提交一轮运行；同会话已有运行时由后端持久化排队。
   */
  const start = async (
    targetSessionId: string,
    input: string,
    mode: "plan" | "yolo",
    selection?: RunModelSelection,
    imageUrls?: string[],
    thinkingLevel?: ThinkingLevel,
    agentId?: string
  ) => {
    const run = await api.runs.start(targetSessionId, input, mode, selection, imageUrls, thinkingLevel, agentId);
    dispatch({ type: "start", run, sessionId: targetSessionId, userInput: input, imageUrls });
  };

  /** 中断指定运行。 */
  const stop = async (runId: string) => {
    await api.runs.stop(runId);
  };

  return { states: state.runs, start, stop, reset: () => dispatch({ type: "reset" }) };
}
