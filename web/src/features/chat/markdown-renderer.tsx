import { useEffect, useId, useState } from "react";
import ReactMarkdown from "react-markdown";
import rehypeKatex from "rehype-katex";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";

export function MarkdownRenderer({ source }: { source: string }) {
  return (
    <div className="markdown-body">
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkMath]}
        rehypePlugins={[rehypeKatex]}
        components={{
          code({ className, children, ...props }) {
            const language = /language-(\w+)/.exec(className ?? "")?.[1];
            const text = String(children).replace(/\n$/, "");
            if (language === "mermaid") return <MermaidDiagram source={text} />;
            if (language) return <pre className="code-block"><span className="code-label">{language}</span><code {...props}>{text}</code></pre>;
            return <code className="inline-code" {...props}>{children}</code>;
          }
        }}
      >
        {source}
      </ReactMarkdown>
    </div>
  );
}

function MermaidDiagram({ source }: { source: string }) {
  const reactId = useId().replace(/:/g, "");
  const [svg, setSvg] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    const timer = window.setTimeout(() => {
      import("mermaid").then(({ default: mermaid }) => {
        mermaid.initialize({ startOnLoad: false, theme: "neutral", securityLevel: "strict" });
        return mermaid.render(`miyu-mermaid-${reactId}`, source);
      })
        .then((result) => { if (active) { setSvg(result.svg); setError(null); } })
        .catch((reason: unknown) => { if (active) setError(reason instanceof Error ? reason.message : "Mermaid 渲染失败"); });
    }, 120);
    return () => { active = false; window.clearTimeout(timer); };
  }, [reactId, source]);
  if (error) return <pre className="asset-source"><code>{source}</code></pre>;
  if (!svg) return <pre className="asset-source"><code>{source}</code></pre>;
  return <div className="mermaid-diagram" dangerouslySetInnerHTML={{ __html: svg }} />;
}
