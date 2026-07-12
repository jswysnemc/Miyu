import { useQuery } from "@tanstack/react-query";
import { ArrowLeft, Ban } from "lucide-react";
import { useEffect, useRef } from "react";
import { api } from "../../api/client";
import type { Subagent } from "../../api/contracts";
import { MarkdownRenderer } from "../chat/markdown-renderer";
import { SubagentProgress } from "./subagent-progress";
import { SubagentStats } from "./subagent-stats";
import { SubagentStatusBadge } from "./subagent-status-badge";
import { SubagentTimeline } from "./subagent-timeline";
import { subagentDuration, subagentTypeLabel } from "./subagent-labels";

type SubagentDetailViewProps = {
  subagent: Subagent;
  onBack: () => void;
  onCancel: (id: string) => void;
};

/**
 * 渲染子智能体详情:元信息、实时进度、流式时间线与 Markdown 结果输出。
 *
 * 运行中每秒轮询详情接口,时间线随执行增量出现;新内容到达时若视口
 * 停留在底部附近则自动跟随滚动。
 *
 * @param props 子智能体列表快照与返回、取消回调
 * @returns 子智能体详情视图
 */
export function SubagentDetailView({ subagent, onBack, onCancel }: SubagentDetailViewProps) {
  const running = subagent.status === "running";
  const detail = useQuery({
    queryKey: ["subagent", subagent.id],
    queryFn: () => api.subagents.detail(subagent.id),
    refetchInterval: running ? 1000 : false
  });
  const scrollRef = useRef<HTMLDivElement>(null);
  const timeline = detail.data?.timeline ?? [];
  const body = subagent.result || subagent.error || "";

  useEffect(() => {
    // 1. 视口停在底部附近时,新时间线内容到达后自动跟随到底
    const node = scrollRef.current;
    if (!node || !running) return;
    const nearBottom = node.scrollHeight - node.scrollTop - node.clientHeight < 120;
    if (nearBottom) node.scrollTop = node.scrollHeight;
  }, [running, timeline.length, body]);

  return (
    <section className="subagent-detail-view">
      <header className="subagent-detail-head">
        <button type="button" className="subagent-detail-back" onClick={onBack}><ArrowLeft size={14} />概览</button>
        <SubagentStatusBadge status={subagent.status} />
        {running && (
          <button type="button" className="subagent-detail-cancel" onClick={() => onCancel(subagent.id)}><Ban size={13} />取消</button>
        )}
      </header>
      <div className="subagent-detail-scroll" ref={scrollRef}>
        <h2 className="subagent-detail-title">{subagent.description}</h2>
        <dl className="subagent-detail-meta">
          <div><dt>类型</dt><dd>{subagentTypeLabel(subagent.subagent_type)}</dd></div>
          <div><dt>用时</dt><dd>{subagentDuration(subagent.started_at, subagent.updated_at)}</dd></div>
          {subagent.last_tool && <div><dt>最近工具</dt><dd>{subagent.last_tool}</dd></div>}
        </dl>
        <SubagentStats subagent={subagent} />
        <SubagentProgress subagent={subagent} />
        <SubagentTimeline entries={timeline} running={running} />
        {body ? (
          <div className="subagent-detail-output"><MarkdownRenderer source={body} /></div>
        ) : (
          timeline.length === 0 && (
            <p className="subagent-detail-pending">{running ? "子智能体正在运行,执行过程会在此实时显示。" : "没有输出。"}</p>
          )
        )}
      </div>
    </section>
  );
}
