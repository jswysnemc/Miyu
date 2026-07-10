import { prettyJson } from "./tool-data";

type GenericToolViewProps = {
  argumentsText: string;
  output: string;
};

/**
 * 渲染未专门适配工具的参数和结果。
 *
 * @param props 工具参数与输出
 * @returns 通用工具详情
 */
export function GenericToolView({ argumentsText, output }: GenericToolViewProps) {
  return (
    <div className="generic-tool-view">
      {argumentsText && <section><span>参数</span><pre className="generic-tool-block"><code>{prettyJson(argumentsText)}</code></pre></section>}
      {output && <section><span>结果</span><pre className="generic-tool-block result"><code>{prettyJson(output)}</code></pre></section>}
    </div>
  );
}
