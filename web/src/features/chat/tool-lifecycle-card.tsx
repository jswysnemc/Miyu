import { Check, ChevronDown, CircleEllipsis, TerminalSquare, X } from "lucide-react";
import { useState } from "react";
import type { ToolLifecycle } from "./run-event-reducer";

export function ToolLifecycleCard({ tool }: { tool: ToolLifecycle }) {
  const [expanded, setExpanded] = useState(tool.name === "edit_file");
  const statusIcon = tool.status === "completed"
    ? <Check size={14} />
    : tool.status === "failed"
      ? <X size={14} />
      : <CircleEllipsis size={14} className="pulse" />;
  const isEdit = tool.name === "edit_file";
  return (
    <section className={`tool-card ${tool.status}`}>
      <button type="button" className="tool-card-head" onClick={() => setExpanded((value) => !value)}>
        <span className="tool-icon"><TerminalSquare size={15} /></span>
        <span className="tool-copy">
          <strong>{readableToolName(tool.name)}</strong>
          <small>{tool.progress || statusLabel(tool.status)}</small>
        </span>
        <span className="tool-status">{statusIcon}<ChevronDown size={14} className={expanded ? "rotate" : ""} /></span>
      </button>
      {expanded && (
        <div className="tool-detail">
          {isEdit ? <EditDiff argumentsText={tool.arguments || tool.argumentsPreview} output={tool.output} /> : (
            <>
              {(tool.arguments || tool.argumentsPreview) && <pre><code>{prettyJson(tool.arguments || tool.argumentsPreview)}</code></pre>}
              {tool.output && <pre className="tool-output"><code>{tool.output}</code></pre>}
            </>
          )}
        </div>
      )}
    </section>
  );
}

function EditDiff({ argumentsText, output }: { argumentsText: string; output: string }) {
  let patch = argumentsText;
  try {
    const parsed = JSON.parse(argumentsText) as { patch?: string };
    patch = parsed.patch ?? argumentsText;
  } catch {
    patch = argumentsText;
  }
  return (
    <div className="inline-diff">
      {patch.split("\n").map((line, index) => (
        <div className={line.startsWith("+") ? "diff-line added" : line.startsWith("-") ? "diff-line removed" : "diff-line"} key={`${index}-${line}`}>
          {line || " "}
        </div>
      ))}
      {output && <pre className="tool-output"><code>{output}</code></pre>}
    </div>
  );
}

function prettyJson(value: string) {
  try { return JSON.stringify(JSON.parse(value), null, 2); } catch { return value; }
}

function readableToolName(name: string) {
  return name.replaceAll("_", " ");
}

function statusLabel(status: ToolLifecycle["status"]) {
  return { preparing: "准备参数", running: "正在执行", completed: "执行完成", failed: "执行失败" }[status];
}
