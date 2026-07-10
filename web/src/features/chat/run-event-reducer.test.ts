import { describe, expect, it } from "vitest";
import type { WebEvent } from "../../api/contracts";
import { initialRunState, runEventReducer } from "./run-event-reducer";

function event(type: string, payload: Record<string, unknown>): WebEvent {
  return { sequence: 1, run_id: "run", workspace_id: "workspace", session_id: "session", timestamp: "now", type, payload };
}

describe("runEventReducer", () => {
  it("streams reasoning and content independently", () => {
    const started = runEventReducer(initialRunState, { type: "start", runId: "run", userInput: "hello" });
    const reasoning = runEventReducer(started, { type: "event", event: event("message.reasoning.delta", { text: "think" }) });
    const content = runEventReducer(reasoning, { type: "event", event: event("message.content.delta", { text: "answer" }) });
    expect(content.reasoning).toBe("think");
    expect(content.content).toBe("answer");
  });

  it("updates one tool card through its lifecycle", () => {
    const preparing = runEventReducer(initialRunState, { type: "event", event: event("tool.call.preparing", { tool_id: "tool", name: "edit_file", arguments_preview: "partial" }) });
    const running = runEventReducer(preparing, { type: "event", event: event("tool.call.started", { tool_id: "tool", name: "edit_file", arguments: "{}" }) });
    const completed = runEventReducer(running, { type: "event", event: event("tool.result", { tool_id: "tool", name: "edit_file", ok: true, output: "ok" }) });
    expect(completed.tools).toHaveLength(1);
    expect(completed.tools[0].status).toBe("completed");
    expect(completed.tools[0].output).toBe("ok");
  });
});
