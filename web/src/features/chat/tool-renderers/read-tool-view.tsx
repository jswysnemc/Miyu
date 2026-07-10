import { FileTypeIcon } from "../../../shared/ui/file-icon";
import { SyntaxHighlighter } from "../syntax-highlighter";
import { parseJsonRecord, prettyJson, stringField } from "./tool-data";

type ReadToolViewProps = {
  argumentsText: string;
  output: string;
};

type ParsedLine = {
  number: number | null;
  text: string;
};

/**
 * read_file 工具专用视图，带语法着色、行号和可点击路径。
 *
 * @param props 读取参数与结果 JSON
 * @returns 文件读取详情
 */
export function ReadToolView({ argumentsText, output }: ReadToolViewProps) {
  const args = parseJsonRecord(argumentsText);
  const result = parseJsonRecord(output);
  const path = stringField(args, "path") || stringField(result, "path");
  const content = stringField(result, "content");
  const isTextPage = stringField(result, "type") === "text-page" && content.length > 0;
  return (
    <div className="read-tool-view">
      {path && (
        <div className="read-file-head">
          <FileTypeIcon name={path} size={13} />
          <OpenFileLink path={path} />
          {result?.offset != null && result?.limit != null && <small>{`第 ${String(result.offset)} 行起`}</small>}
        </div>
      )}
      {isTextPage
        ? <FileContentBlock path={path} content={content} />
        : output && <pre className="generic-tool-block result"><code>{prettyJson(output)}</code></pre>}
    </div>
  );
}

/**
 * 可点击文件路径，点击时派发全局打开文件事件。
 *
 * @param props path 为原样保留的文件路径
 * @returns 可交互路径元素
 */
export function OpenFileLink({ path }: { path: string }) {
  // 1. 点击时派发 miyu:open-file 事件由工作区监听打开
  const onOpen = () => {
    window.dispatchEvent(new CustomEvent("miyu:open-file", { detail: { path } }));
  };
  return (
    <button type="button" className="open-file-link" onClick={onOpen} title="在编辑器中打开">
      {path}
    </button>
  );
}

/**
 * 按扩展名着色渲染文件内容，带行号列和高度上限。
 *
 * @param props path 用于推断语言，content 为带行号前缀的文本
 * @returns 着色后的文件内容块
 */
function FileContentBlock({ path, content }: { path: string; content: string }) {
  // 1. 拆出后端 "行号: 内容" 前缀
  const lines = content.split("\n").map(parseNumberedLine);
  const source = lines.map((line) => line.text).join("\n");
  const language = path.includes(".") ? path.split(".").pop() : undefined;
  return (
    <div className="read-file-content">
      <div className="read-file-gutter" aria-hidden>
        {lines.map((line, index) => <span key={index}>{line.number ?? ""}</span>)}
      </div>
      <pre className="read-file-code"><SyntaxHighlighter language={language} source={source} /></pre>
    </div>
  );
}

/**
 * 解析单行的行号前缀。
 *
 * @param line 形如 "12: 内容" 的文本
 * @returns 行号与正文，无前缀时行号为空
 */
function parseNumberedLine(line: string): ParsedLine {
  const match = /^(\d+): (.*)$/s.exec(line);
  if (!match) return { number: null, text: line };
  return { number: Number(match[1]), text: match[2] };
}
