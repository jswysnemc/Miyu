// ============================================================
// 记忆类工具渲染器：remember_fact/recall_memory/search_evicted
// 摘要显示记忆要点，输出区文本渲染
// ============================================================

import {
  toolIcons,
  parseArgs,
  renderTextOutput,
  renderErrorOutput,
  clipSummary,
} from "./common.js";

const remember = {
  icon: toolIcons.brain,
  /**
   * 摘要：显示记忆事实
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const fact = args.fact || args.content || args.key || "";
    return fact ? `记忆 ${clipSummary(fact, 70)}` : "记录事实";
  },
  /**
   * 渲染输出
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 3000);
  },
};

const recall = {
  icon: toolIcons.brain,
  /**
   * 摘要：显示召回查询
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const q = args.query || args.keyword || args.key || "";
    return q ? `召回记忆 "${clipSummary(q, 60)}"` : "召回记忆";
  },
  /**
   * 渲染输出
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 4000);
  },
};

const evicted = {
  icon: toolIcons.brain,
  /**
   * 摘要：显示检索关键词
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const q = args.query || args.keyword || "";
    return q ? `检索裁剪上下文 "${clipSummary(q, 50)}"` : "检索裁剪上下文";
  },
  /**
   * 渲染输出
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 4000);
  },
};

export default { remember, recall, evicted };
