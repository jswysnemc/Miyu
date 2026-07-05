// ============================================================
// 网络类工具渲染器：web_search/web_fetch/search_web_images
// 摘要显示查询词或 URL，输出区渲染搜索结果摘要
// ============================================================

import {
  toolIcons,
  parseArgs,
  renderTextOutput,
  renderErrorOutput,
  clipSummary,
} from "./common.js";

const search = {
  icon: toolIcons.globe,
  /**
   * 摘要：显示查询词
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const q = args.query || args.q || args.keyword || "";
    return q ? `搜索 "${clipSummary(q, 70)}"` : "网络搜索";
  },
  /**
   * 渲染输出：搜索结果文本
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 6000);
  },
};

const fetch = {
  icon: toolIcons.globe,
  /**
   * 摘要：显示抓取的 URL
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const url = args.url || args.uri || "";
    return url ? `抓取 ${clipSummary(url, 90)}` : "抓取网页";
  },
  /**
   * 渲染输出：网页内容
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 8000);
  },
};

const images = {
  icon: toolIcons.image,
  /**
   * 摘要：显示搜图关键词
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const q = args.query || args.keyword || "";
    return q ? `搜图 "${clipSummary(q, 70)}"` : "搜索图片";
  },
  /**
   * 渲染输出：图片搜索结果
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 4000);
  },
};

export default { search, fetch, images };
