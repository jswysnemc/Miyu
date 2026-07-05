// ============================================================
// 知识库类工具渲染器：upload_kb/read_kb/search_kb
// 摘要显示文件名或查询词，输出区文本渲染
// ============================================================

import {
  toolIcons,
  parseArgs,
  renderTextOutput,
  renderErrorOutput,
  clipSummary,
} from "./common.js";

const upload = {
  icon: toolIcons.book,
  /**
   * 摘要：显示上传文件名
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const f = args.file || args.filename || args.path || "";
    return f ? `上传知识库 ${clipSummary(f, 70)}` : "上传知识库";
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

const read = {
  icon: toolIcons.book,
  /**
   * 摘要：显示阅读的知识库条目
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const t = args.topic || args.title || args.path || "";
    return t ? `阅读知识库 ${clipSummary(t, 70)}` : "阅读知识库";
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

const search = {
  icon: toolIcons.book,
  /**
   * 摘要：显示搜索关键词
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const q = args.query || args.keyword || args.topic || "";
    return q ? `检索知识库 "${clipSummary(q, 60)}"` : "检索知识库";
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

export default { upload, read, search };
