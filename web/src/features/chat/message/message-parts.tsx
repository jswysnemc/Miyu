import type { LiveMessagePart } from "../run-event-reducer";
import { MarkdownRenderer } from "../markdown-renderer";
import { ReasoningBlock } from "../reasoning-block";
import { ToolLifecycleCard } from "../tool-lifecycle-card";

/**
 * 按消息部件顺序渲染思考、正文和工具调用。
 *
 * @param props 有序消息部件及实时运行状态
 * @returns 嵌入同一助手消息中的部件列表
 */
export function MessageParts({ parts, live }: { parts: LiveMessagePart[]; live?: boolean }) {
  return (
    <div className="message-parts">
      {parts.map((part) => {
        if (part.type === "reasoning") {
          return <ReasoningBlock key={part.id} source={part.source} live={live && !part.endedAt} startedAt={part.startedAt} endedAt={part.endedAt} />;
        }
        if (part.type === "tool") return <ToolLifecycleCard key={part.id} tool={part.tool} />;
        return <MarkdownRenderer key={part.id} source={part.source} />;
      })}
    </div>
  );
}
