import ReactMarkdown from "react-markdown";
import rehypeKatex from "rehype-katex";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import { MarkdownCodeBlock } from "./markdown-code-block";
import { MermaidDiagram } from "./mermaid-diagram";
import "./markdown-renderer.css";

/**
 * 渲染支持 GFM、数学公式、代码块和 Mermaid 的 Markdown 内容。
 *
 * @param props Markdown 源文本
 * @returns Markdown 内容
 */
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
            if (language || text.includes("\n")) return <MarkdownCodeBlock language={language} source={text} />;
            return <code className="inline-code" {...props}>{children}</code>;
          },
          a({ children, ...props }) {
            return <a {...props} target="_blank" rel="noreferrer">{children}</a>;
          },
          table({ children }) {
            return <div className="markdown-table-wrap"><table>{children}</table></div>;
          },
          img({ alt, ...props }) {
            return <img {...props} alt={alt ?? ""} loading="lazy" />;
          }
        }}
      >
        {source}
      </ReactMarkdown>
    </div>
  );
}
