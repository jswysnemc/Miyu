// ============================================================
// load 工具渲染器：渐进式工具加载
// 摘要显示加载的工具/组/skill 名称
// ============================================================

import {
  toolIcons,
  parseArgs,
  renderTextOutput,
  renderErrorOutput,
  clipSummary,
} from "./common.js";

const load = {
  icon: toolIcons.plug,
  /**
   * 摘要：显示加载目标
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    // 1. 优先显示具体加载项
    const tool = args.tool || args.name;
    const group = args.group;
    const skill = args.skill;
    const target = tool || group || skill || args.target || "";
    return target ? `加载 ${clipSummary(target, 60)}` : "加载工具";
  },
  /**
   * 渲染输出：加载结果
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 3000);
  },
};

export default load;
