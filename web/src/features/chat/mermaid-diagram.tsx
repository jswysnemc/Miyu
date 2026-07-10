import { Check, Code2, Copy, Eye } from "lucide-react";
import { useEffect, useId, useState } from "react";

/**
 * 渲染可在图表预览和 Mermaid 源码之间切换的内容块。
 *
 * @param props Mermaid 源码
 * @returns Mermaid 图表或源码
 */
export function MermaidDiagram({ source }: { source: string }) {
  const reactId = useId().replace(/:/g, "");
  const [svg, setSvg] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<"preview" | "source">("preview");
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let active = true;
    const timer = window.setTimeout(() => {
      import("mermaid").then(({ default: mermaid }) => {
        mermaid.initialize({ startOnLoad: false, theme: "neutral", securityLevel: "strict" });
        return mermaid.render(`miyu-mermaid-${reactId}`, source);
      })
        .then((result) => {
          if (!active) return;
          setSvg(result.svg);
          setError(null);
        })
        .catch((reason: unknown) => {
          if (active) setError(reason instanceof Error ? reason.message : "Mermaid 渲染失败");
        });
    }, 120);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [reactId, source]);

  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), 1_600);
    return () => window.clearTimeout(timer);
  }, [copied]);

  /** 复制 Mermaid 源码。 */
  const copySource = async () => {
    await navigator.clipboard.writeText(source);
    setCopied(true);
  };

  return (
    <div className="mermaid-wrapper">
      <div className="mermaid-toolbar">
        <span>mermaid</span>
        <div className="mermaid-view-switcher">
          <button type="button" className={view === "preview" ? "active" : ""} onClick={() => setView("preview")}><Eye size={13} />预览</button>
          <button type="button" className={view === "source" ? "active" : ""} onClick={() => setView("source")}><Code2 size={13} />源码</button>
        </div>
        <button type="button" className="mermaid-copy" onClick={() => void copySource()}>{copied ? <Check size={13} /> : <Copy size={13} />}{copied ? "已复制" : "复制"}</button>
      </div>
      {view === "source" || error || !svg
        ? <pre className="mermaid-source"><code>{source}</code></pre>
        : <div className="mermaid-preview" dangerouslySetInnerHTML={{ __html: svg }} />}
      {error && <div className="mermaid-error">{error}</div>}
    </div>
  );
}
