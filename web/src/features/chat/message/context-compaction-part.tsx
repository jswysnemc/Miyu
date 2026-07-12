import { Archive, Loader2 } from "lucide-react";
import type { LiveMessagePart } from "../run-event-reducer";

type CompactionPart = Extract<LiveMessagePart, { type: "compaction" }>;

/**
 * 渲染运行期间的上下文压缩状态。
 *
 * @param props 压缩状态部件
 * @returns 与消息时间线一致的紧凑状态行
 */
export function ContextCompactionPart({ part }: { part: CompactionPart }) {
  const running = part.status === "running";
  const text = running
    ? `正在压缩 ${part.turnCount} 轮旧上下文`
    : part.applied
      ? `已压缩 ${part.turnCount} 轮旧上下文`
      : "本次上下文压缩未应用";
  return (
    <div className={`context-compaction-part${running ? " running" : ""}`}>
      {running ? <Loader2 size={14} className="spin" /> : <Archive size={14} />}
      <span>{text}</span>
    </div>
  );
}
