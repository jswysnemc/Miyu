import { ArrowLeft, Ban } from "lucide-react";
import type { Subagent } from "../../api/contracts";
import { MarkdownRenderer } from "../chat/markdown-renderer";
import { SubagentProgress } from "./subagent-progress";
import { SubagentStats } from "./subagent-stats";
import { SubagentStatusBadge } from "./subagent-status-badge";
import { subagentDuration, subagentTypeLabel } from "./subagent-labels";

type SubagentDetailViewProps = {
  subagent: Subagent;
  onBack: () => void;
  onCancel: (id: string) => void;
};

/**
 * 渲染子智能体详情:元信息、实时进度、统计与 Markdown 结果输出。
 *
 * 结果输出复用主对话的 MarkdownRenderer,与聊天区呈现保持一致。
 *
 * @param props 子智能体数据与返回、取消回调
 * @returns 子智能体详情视图
 */
export function SubagentDetailView({ subagent, onBack, onCancel }: SubagentDetailViewProps) {
  const running = subagent.status === "running";
  const body = subagent.result || subagent.error || "";
  return (
    <section className="subagent-detail-view">
      <header className="subagent-detail-head">
        <button type="button" className="subagent-detail-back" onClick={onBack}><ArrowLeft size={14} />概览</button>
        <SubagentStatusBadge status={subagent.status} />
        {running && (
          <button type="button" className="subagent-detail-cancel" onClick={() => onCancel(subagent.id)}><Ban size={13} />取消</button>
        )}
      </header>
      <div className="subagent-detail-scroll">
        <h2 className="subagent-detail-title">{subagent.description}</h2>
        <dl className="subagent-detail-meta">
          <div><dt>类型</dt><dd>{subagentTypeLabel(subagent.subagent_type)}</dd></div>
          <div><dt>用时</dt><dd>{subagentDuration(subagent.started_at, subagent.updated_at)}</dd></div>
          {subagent.last_tool && <div><dt>最近工具</dt><dd>{subagent.last_tool}</dd></div>}
        </dl>
        <SubagentStats subagent={subagent} />
        <SubagentProgress subagent={subagent} />
        {body ? (
          <div className="subagent-detail-output"><MarkdownRenderer source={body} /></div>
        ) : (
          <p className="subagent-detail-pending">{running ? "子智能体正在运行,结果就绪后在此显示。" : "没有输出。"}</p>
        )}
      </div>
    </section>
  );
}
