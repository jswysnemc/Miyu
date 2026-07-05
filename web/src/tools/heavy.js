// ============================================================
// 重型与子代理工具渲染器：deep_research/deep_diagnose/linux_game/task
// 摘要显示任务主题，输出区以 Markdown 渲染报告
// ============================================================

import {
  toolIcons,
  parseArgs,
  renderMarkdownOutput,
  renderTextOutput,
  renderErrorOutput,
  clipSummary,
} from "./common.js";

const research = {
  icon: toolIcons.rocket,
  /**
   * 摘要：显示研究主题
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const t = args.topic || args.query || args.subject || "";
    return t ? `深度研究 ${clipSummary(t, 60)}` : "深度研究";
  },
  /**
   * 渲染输出：Markdown 报告
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderMarkdownOutput(ctx.bodyEl, ctx.output, 12000);
  },
};

const diagnose = {
  icon: toolIcons.rocket,
  /**
   * 摘要：显示诊断目标
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const t = args.topic || args.target || args.problem || "";
    return t ? `深度诊断 ${clipSummary(t, 60)}` : "深度诊断";
  },
  /**
   * 渲染输出：Markdown 报告
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderMarkdownOutput(ctx.bodyEl, ctx.output, 12000);
  },
};

const game = {
  icon: toolIcons.dice,
  /**
   * 摘要：显示游戏主题
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const t = args.topic || args.game || args.theme || "";
    return t ? `Linux 游戏 ${clipSummary(t, 60)}` : "Linux 游戏";
  },
  /**
   * 渲染输出：Markdown 报告
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderMarkdownOutput(ctx.bodyEl, ctx.output, 10000);
  },
};

const task = {
  icon: toolIcons.layers,
  /**
   * 摘要：显示子代理任务
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const p = args.prompt || args.task || args.description || "";
    return p ? `子代理 ${clipSummary(p, 70)}` : "子代理任务";
  },
  /**
   * 渲染输出：任务结果
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 6000);
  },
};

export default { research, diagnose, game, task };
