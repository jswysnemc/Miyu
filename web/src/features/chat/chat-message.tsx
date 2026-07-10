import type { HistoryEntry, SessionTimelineTurn, TimelineToolEntry } from "../../api/contracts";
import type { LiveRunState } from "./run-event-reducer";
import type { LiveMessagePart } from "./run-event-reducer";
import { MessageParts } from "./message/message-parts";

/**
 * 渲染一条历史消息。
 *
 * @param props 历史消息内容
 * @returns 用户或助手消息
 */
export function HistoryMessage({ message }: { message: HistoryEntry }) {
  if (message.role === "user") return <article className="message user-message"><div className="message-content">{message.content}</div></article>;
  return (
    <article className="message assistant-message">
      <MessageParts parts={historyMessageParts(message)} />
    </article>
  );
}

/**
 * 渲染一个包含结构化工具历史的完整对话轮次。
 *
 * @param props 会话时间线轮次
 * @returns 用户消息、工具调用和助手消息
 */
export function HistoryTurn({ turn }: { turn: SessionTimelineTurn }) {
  return (
    <>
      <article className="message user-message"><div className="message-content">{turn.user.content}</div></article>
      <article className="message assistant-message">
        <MessageParts parts={historyTurnParts(turn)} />
      </article>
    </>
  );
}

/**
 * 渲染当前正在流式生成的用户输入和助手回复。
 *
 * @param props 运行状态和运行标记
 * @returns 当前运行消息组
 */
export function LiveRunMessage({ state, running }: { state: LiveRunState; running: boolean }) {
  return (
    <>
      <article className="message user-message">
        <div className="user-message-stack">
          <div className="message-content">{state.userInput}</div>
          {state.imageUrls.length > 0 && <div className="user-attachments">{state.imageUrls.map((url, index) => <img className="user-attachment" src={url} alt={`用户附件 ${index + 1}`} key={`${index}-${url.slice(-24)}`} />)}</div>}
        </div>
      </article>
      <article className="message assistant-message live-message">
        <MessageParts parts={state.parts} live={running} />
        {running && !state.content && !state.reasoning && state.tools.length === 0 && <div className="response-pulse"><span /><span /><span /></div>}
        {state.error && <div className="run-error">{state.error}</div>}
      </article>
    </>
  );
}

/**
 * 将旧版消息转换为统一消息部件。
 *
 * @param message 历史消息
 * @returns 有序消息部件
 */
function historyMessageParts(message: HistoryEntry): LiveMessagePart[] {
  const parts: LiveMessagePart[] = [];
  if (message.reasoning) parts.push({ id: `reasoning-${message.timestamp}`, type: "reasoning", source: message.reasoning, startedAt: "" });
  if (message.content) parts.push({ id: `text-${message.timestamp}`, type: "text", source: message.content });
  return parts;
}

/**
 * 将会话轮次转换为同一消息内的有序部件。
 *
 * @param turn 会话时间线轮次
 * @returns 思考、工具和正文部件
 */
function historyTurnParts(turn: SessionTimelineTurn): LiveMessagePart[] {
  const parts: LiveMessagePart[] = [];
  if (turn.assistant.reasoning) {
    parts.push({ id: `${turn.turn_id}-reasoning`, type: "reasoning", source: turn.assistant.reasoning, startedAt: "" });
  }
  const tools = [...turn.tools].sort((left, right) => left.created_at.localeCompare(right.created_at));
  for (const tool of tools) parts.push({ id: `${turn.turn_id}-${tool.id}`, type: "tool", tool: timelineTool(tool) });
  if (turn.assistant.content) parts.push({ id: `${turn.turn_id}-text`, type: "text", source: turn.assistant.content });
  return parts;
}

/**
 * 将后端时间线工具记录转换为统一生命周期状态。
 *
 * @param tool 时间线工具记录
 * @returns 工具生命周期状态
 */
function timelineTool(tool: TimelineToolEntry): LiveRunState["tools"][number] {
  return {
    id: tool.id,
    name: tool.name,
    argumentsPreview: tool.arguments,
    arguments: tool.arguments,
    progress: "",
    output: tool.output || tool.error || "",
    status: tool.status
  };
}
