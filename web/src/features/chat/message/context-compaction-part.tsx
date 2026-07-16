import { Archive, Loader2 } from "lucide-react";
import type { LiveMessagePart } from "../run-event-reducer";
import { MarkdownRenderer } from "../markdown-renderer";

type CompactionPart = Extract<LiveMessagePart, { type: "compaction" }>;

/**
 * 渲染运行期间的上下文压缩状态，并在应用成功后展示压缩摘要。
 *
 * @param props 压缩状态部件
 * @returns 状态行；成功应用时附带分割线与摘要内容
 */
export function ContextCompactionPart({ part }: { part: CompactionPart }) {
  const running = part.status === "running";
  const text = running
    ? `正在压缩 ${part.turnCount} 轮旧上下文`
    : part.applied
      ? `已压缩 ${part.turnCount} 轮旧上下文`
      : "本次上下文压缩未应用";
  const summary = part.status === "completed" && part.applied && part.summary?.trim()
    ? part.summary.trim()
    : null;

  return (
    <div className={`context-compaction-block${running ? " running" : ""}`}>
      <div className="context-compaction-part">
        {running ? <Loader2 size={14} className="spin" /> : <Archive size={14} />}
        <span>{text}</span>
      </div>
      {summary && (
        <>
          <div className="context-compaction-divider" role="separator" aria-label="压缩内容分割线">
            <span className="context-compaction-divider-line" />
            <span className="context-compaction-divider-label">压缩内容</span>
            <span className="context-compaction-divider-line" />
          </div>
          <div className="context-compaction-summary">
            <MarkdownRenderer source={summary} />
          </div>
        </>
      )}
    </div>
  );
}
