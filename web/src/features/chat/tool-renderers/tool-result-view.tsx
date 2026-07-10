import { EditToolView } from "./edit-tool-view";
import { GenericToolView } from "./generic-tool-view";
import { ShellToolView } from "./shell-tool-view";

type ToolResultViewProps = {
  name: string;
  argumentsText: string;
  output: string;
};

/**
 * 按工具类型选择专业结果渲染器。
 *
 * @param props 工具名称、参数和输出
 * @returns 工具结果视图
 */
export function ToolResultView({ name, argumentsText, output }: ToolResultViewProps) {
  if (name === "run_command" || name.includes("background_command")) {
    return <ShellToolView argumentsText={argumentsText} output={output} />;
  }
  if (name === "edit_file" || name === "apply_patch") {
    return <EditToolView argumentsText={argumentsText} output={output} />;
  }
  return <GenericToolView argumentsText={argumentsText} output={output} />;
}
