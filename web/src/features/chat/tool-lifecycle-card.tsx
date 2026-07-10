import { Check, ChevronDown, CircleEllipsis, FilePenLine, FileSearch, Search, TerminalSquare, Wrench, X } from "lucide-react";
import { useState } from "react";
import type { ToolLifecycle } from "./run-event-reducer";
import { toolSummary } from "./tool-renderers/tool-data";
import { ToolResultView } from "./tool-renderers/tool-result-view";
import "./tool-renderers/tool-renderers.css";

/**
 * 渲染一项实时或历史工具生命周期。
 *
 * @param props 工具生命周期状态
 * @returns 可折叠工具卡片
 */
export function ToolLifecycleCard({ tool }: { tool: ToolLifecycle }) {
  const [expanded, setExpanded] = useState(tool.status === "failed");
  const statusIcon = tool.status === "completed"
    ? <Check size={14} />
    : tool.status === "failed"
      ? <X size={14} />
      : <CircleEllipsis size={14} className="pulse" />;
  const argumentsText = tool.arguments || tool.argumentsPreview;
  const summary = tool.progress || toolSummary(tool.name, argumentsText) || statusLabel(tool.status);
  return (
    <section className={`tool-card tool-inline-row ${tool.status}`}>
      <button type="button" className="tool-card-head" onClick={() => setExpanded((value) => !value)} aria-expanded={expanded}>
        <span className="tool-icon"><ToolIcon name={tool.name} /></span>
        <span className="tool-copy"><strong>{readableToolName(tool.name)}</strong><small>{summary}</small></span>
        <span className="tool-status">{statusIcon}<ChevronDown size={14} className={expanded ? "rotate" : ""} /></span>
      </button>
      {expanded && <div className="tool-detail"><ToolResultView name={tool.name} argumentsText={argumentsText} output={tool.output} /></div>}
    </section>
  );
}

/**
 * 按工具语义返回图标。
 *
 * @param props 工具名称
 * @returns 工具图标
 */
function ToolIcon({ name }: { name: string }) {
  if (name === "run_command" || name.includes("command")) return <TerminalSquare size={15} />;
  if (name === "edit_file" || name === "apply_patch") return <FilePenLine size={15} />;
  if (name === "read_file") return <FileSearch size={15} />;
  if (name === "grep" || name === "glob") return <Search size={15} />;
  return <Wrench size={15} />;
}

/**
 * 将工具标识转换为可读名称。
 *
 * @param name 工具标识
 * @returns 可读名称
 */
function readableToolName(name: string): string {
  const labels: Record<string, string> = {
    run_command: "Shell",
    edit_file: "Edit",
    apply_patch: "Patch",
    read_file: "Read",
    grep: "Search",
    glob: "Files"
  };
  return labels[name] ?? name.replaceAll("_", " ");
}

/**
 * 返回工具状态中文标签。
 *
 * @param status 工具状态
 * @returns 状态标签
 */
function statusLabel(status: ToolLifecycle["status"]): string {
  return { preparing: "准备参数", running: "正在执行", completed: "执行完成", failed: "执行失败" }[status];
}
