// ============================================================
// 默认工具渲染器：未注册工具的兜底渲染
// 摘要显示工具名，输出区纯文本
// ============================================================

import {
  toolIcons,
  parseArgs,
  renderTextOutput,
  renderErrorOutput,
  clipSummary,
} from "./common.js";

const defaultRenderer = {
  icon: toolIcons.package,
  /**
   * 摘要：显示工具名或参数预览
   * @param {Object} args - 参数
   * @param {Object} ctx - 上下文 { name }
   * @returns {string}
   */
  summarize(args, ctx) {
    // 1. 尝试从参数中提取首个字符串值作为摘要
    const first = Object.values(args).find(
      (v) => typeof v === "string" && v.length > 0
    );
    if (first) return clipSummary(first, 70);
    return ctx && ctx.name ? ctx.name : "工具调用";
  },
  /**
   * 渲染输出：纯文本
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 4000);
  },
};

export default defaultRenderer;
