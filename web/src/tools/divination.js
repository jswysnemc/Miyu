// ============================================================
// 玄学类工具渲染器：tarot/zhouyi/fortune_lot/dice
// 摘要显示占卜主题，输出区文本渲染
// ============================================================

import {
  toolIcons,
  parseArgs,
  renderTextOutput,
  renderErrorOutput,
  clipSummary,
} from "./common.js";

const tarot = {
  icon: toolIcons.sparkles,
  /**
   * 摘要：显示占卜问题
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const q = args.question || args.query || args.spread || "";
    return q ? `塔罗 ${clipSummary(q, 60)}` : "塔罗占卜";
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

const zhouyi = {
  icon: toolIcons.sparkles,
  /**
   * 摘要：显示占卜问题
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const q = args.question || args.query || "";
    return q ? `周易 ${clipSummary(q, 60)}` : "周易占卜";
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

const fortune = {
  icon: toolIcons.sparkles,
  /**
   * 摘要
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const q = args.question || args.topic || "";
    return q ? `求签 ${clipSummary(q, 60)}` : "求签";
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

const dice = {
  icon: toolIcons.dice,
  /**
   * 摘要：显示骰子规格
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const n = args.count || args.sides || args.times || "";
    return n ? `掷骰 ${n}` : "掷骰子";
  },
  /**
   * 渲染输出
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 2000);
  },
};

export default { tarot, zhouyi, fortune, dice };
