import { FileCheck2 } from "lucide-react";
import { DiffView } from "./diff-view";
import { parseJsonRecord, prettyJson, stringField } from "./tool-data";

type EditToolViewProps = {
  argumentsText: string;
  output: string;
};

type ChangedFile = {
  path?: string;
  action?: string;
  added?: number;
  removed?: number;
};

/**
 * 渲染文件修改 patch、变更文件和行数统计。
 *
 * @param props 编辑参数与结果
 * @returns 文件修改详情
 */
export function EditToolView({ argumentsText, output }: EditToolViewProps) {
  const args = parseJsonRecord(argumentsText);
  const result = parseJsonRecord(output);
  const patch = stringField(args, "patch");
  const path = stringField(args, "path");
  const changedFiles = Array.isArray(result?.changed_files) ? result.changed_files as ChangedFile[] : [];
  return (
    <div className="edit-tool-view">
      {changedFiles.length > 0 && (
        <div className="changed-file-list">
          {changedFiles.map((file, index) => (
            <div className="changed-file" key={`${file.path}-${index}`}>
              <FileCheck2 size={14} />
              <span><strong>{file.path || "未知文件"}</strong><small>{file.action || "Edited"}</small></span>
              <span className="changed-file-stats"><b>+{file.added ?? 0}</b><i>-{file.removed ?? 0}</i></span>
            </div>
          ))}
        </div>
      )}
      {path && !patch && <div className="legacy-edit-path"><span>文件</span><code>{path}</code></div>}
      {patch ? <DiffView source={patch} /> : argumentsText && <pre className="generic-tool-block"><code>{prettyJson(argumentsText)}</code></pre>}
      {output && !result && <pre className="generic-tool-block result"><code>{output}</code></pre>}
    </div>
  );
}
