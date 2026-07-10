import { FileTypeIcon } from "../../../shared/ui/file-icon";
import { parseDiff } from "./diff-parser";
import type { DiffFile, DiffLine } from "./diff-parser";

type DiffViewProps = {
  source: string;
};

/**
 * 以 IDE 风格渲染统一 Diff 或 Codex patch 文本。
 *
 * @param props Diff 源文本
 * @returns 按文件分块、带双行号列的 Diff 视图
 */
export function DiffView({ source }: DiffViewProps) {
  const files = parseDiff(source);
  if (files.length === 0) return null;
  return (
    <div className="structured-diff" role="region" aria-label="文件差异">
      {files.map((file, index) => <DiffFileBlock file={file} key={`${file.path}-${index}`} />)}
    </div>
  );
}

/**
 * 渲染单个文件的差异块，含文件名条与增删统计徽标。
 *
 * @param props 解析后的文件差异
 * @returns 文件差异块
 */
function DiffFileBlock({ file }: { file: DiffFile }) {
  return (
    <section className="diff-file">
      <header className="diff-file-head">
        <FileTypeIcon name={file.path || "diff"} size={13} />
        <strong>{file.path || "变更片段"}</strong>
        <small>{file.action}</small>
        <span className="diff-file-stats">
          {file.added > 0 && <b>+{file.added}</b>}
          {file.removed > 0 && <i>-{file.removed}</i>}
        </span>
      </header>
      <div className="diff-file-lines">
        {file.lines.map((line, index) => <DiffLineRow line={line} key={index} />)}
      </div>
    </section>
  );
}

/**
 * 渲染一行差异内容，删除行显示旧行号、新增行显示新行号。
 *
 * @param props 解析后的差异行
 * @returns 差异行元素
 */
function DiffLineRow({ line }: { line: DiffLine }) {
  // 1. hunk 头渲染为弱化分隔条
  if (line.kind === "hunk") {
    return (
      <div className="diff-row hunk">
        <span className="diff-gutter" />
        <span className="diff-gutter" />
        <code>{line.text}</code>
      </div>
    );
  }
  // 2. 内容行渲染双行号列与带标记的代码
  const marker = line.kind === "added" ? "+" : line.kind === "removed" ? "-" : " ";
  return (
    <div className={`diff-row ${line.kind}`}>
      <span className="diff-gutter">{line.oldLine ?? ""}</span>
      <span className="diff-gutter">{line.newLine ?? ""}</span>
      <code><span className="diff-marker">{marker}</span>{line.text || " "}</code>
    </div>
  );
}
