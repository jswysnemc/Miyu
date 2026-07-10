type DiffViewProps = {
  source: string;
};

/**
 * 渲染统一 Diff 或 Codex patch 文本。
 *
 * @param props Diff 源文本
 * @returns 带行类型和行号的 Diff 视图
 */
export function DiffView({ source }: DiffViewProps) {
  const lines = source.replaceAll("\r\n", "\n").split("\n");
  return (
    <div className="structured-diff" role="region" aria-label="文件差异">
      {lines.map((line, index) => (
        <div className={`structured-diff-line ${diffLineKind(line)}`} key={`${index}-${line}`}>
          <span className="structured-diff-number">{index + 1}</span>
          <code>{line || " "}</code>
        </div>
      ))}
    </div>
  );
}

/**
 * 判断 Diff 行的展示类型。
 *
 * @param line Diff 行文本
 * @returns CSS 类型名称
 */
function diffLineKind(line: string): string {
  if (line.startsWith("diff --git") || line.startsWith("*** Begin Patch") || line.startsWith("*** End Patch") || /^\*\*\* (Add|Delete|Update) File: /.test(line)) return "file";
  if (line.startsWith("@@")) return "hunk";
  if (line.startsWith("+++") || line.startsWith("---")) return "meta";
  if (line.startsWith("+")) return "added";
  if (line.startsWith("-")) return "removed";
  return "context";
}
