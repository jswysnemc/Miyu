// ============================================================
// 系统类工具渲染器：alarm/weather/exchange_rate/man/archlinux
// 摘要显示关键参数，输出区文本渲染
// ============================================================

import {
  toolIcons,
  parseArgs,
  renderTextOutput,
  renderErrorOutput,
  clipSummary,
} from "./common.js";

const alarm = {
  icon: toolIcons.clock,
  /**
   * 摘要：显示闹钟时间
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const t = args.time || args.at || args.when || "";
    const m = args.message || args.label || "";
    return t ? `闹钟 ${clipSummary(t, 30)}${m ? ` ${clipSummary(m, 40)}` : ""}` : "设置闹钟";
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

const weather = {
  icon: toolIcons.cloud,
  /**
   * 摘要：显示城市
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const c = args.city || args.location || args.place || "";
    return c ? `天气 ${clipSummary(c, 50)}` : "查询天气";
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

const exchange = {
  icon: toolIcons.coins,
  /**
   * 摘要：显示币种
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const base = args.base || args.from || args.currency || "";
    return base ? `汇率 ${clipSummary(base, 40)}` : "查询汇率";
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

const man = {
  icon: toolIcons.book,
  /**
   * 摘要：显示命令名
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const c = args.command || args.name || args.cmd || "";
    return c ? `man ${clipSummary(c, 50)}` : "查看手册";
  },
  /**
   * 渲染输出
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 6000);
  },
};

const arch = {
  icon: toolIcons.package,
  /**
   * 摘要：显示搜索词
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const q = args.query || args.package || args.keyword || "";
    return q ? `archlinux ${clipSummary(q, 50)}` : "Arch 包查询";
  },
  /**
   * 渲染输出
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 6000);
  },
};

export default { alarm, weather, exchange, man, arch };
