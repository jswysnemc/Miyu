// ============================================================
// 文件类工具渲染器：read/write/edit/glob/grep/trash
// 摘要突出文件路径，输出按文本或 diff 风格渲染
// ============================================================

import {
  toolIcons,
  parseArgs,
  renderTextOutput,
  renderErrorOutput,
  clipSummary,
} from "./common.js";

/**
 * 提取路径参数，兼容 path/file/file_path 等字段名
 * @param {Object} args - 参数对象
 * @returns {string} 路径文本
 */
function pickPath(args) {
  return args.path || args.file_path || args.file || args.target || "";
}

const read = {
  icon: toolIcons.file,
  /**
   * 摘要：显示文件路径
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const p = pickPath(args);
    return p ? clipSummary(p, 100) : "读取文件";
  },
  /**
   * 渲染输出：文件内容纯文本
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 6000);
  },
};

const write = {
  icon: toolIcons.edit,
  /**
   * 摘要：显示写入路径
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const p = pickPath(args);
    return p ? `写入 ${clipSummary(p, 90)}` : "写入文件";
  },
  /**
   * 渲染输出：写入内容预览
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 4000);
  },
};

const edit = {
  icon: toolIcons.edit,
  /**
   * 摘要：显示编辑路径
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const p = pickPath(args);
    return p ? `编辑 ${clipSummary(p, 90)}` : "编辑文件";
  },
  /**
   * 渲染输出：编辑结果
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 4000);
  },
};

const glob = {
  icon: toolIcons.search,
  /**
   * 摘要：显示匹配模式
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const pattern = args.pattern || args.glob || "";
    const scope = pickPath(args);
    return pattern
      ? `${clipSummary(pattern, 60)}${scope ? ` in ${clipSummary(scope, 40)}` : ""}`
      : "glob 匹配";
  },
  /**
   * 渲染输出：匹配到的文件列表
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 6000);
  },
};

const grep = {
  icon: toolIcons.search,
  /**
   * 摘要：显示搜索模式
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const pattern = args.pattern || args.query || args.regex || "";
    const scope = pickPath(args);
    return pattern
      ? `"${clipSummary(pattern, 50)}"${scope ? ` in ${clipSummary(scope, 40)}` : ""}`
      : "grep 搜索";
  },
  /**
   * 渲染输出：搜索结果
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 6000);
  },
};

const trash = {
  icon: toolIcons.trash,
  /**
   * 摘要：显示删除目标
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const paths = args.paths || args.path || args.files;
    if (Array.isArray(paths)) {
      return `删除 ${paths.length} 项`;
    }
    const p = pickPath(args);
    return p ? `删除 ${clipSummary(p, 90)}` : "删除文件";
  },
  /**
   * 渲染输出：删除结果
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 2000);
  },
};

export default { read, write, edit, glob, grep, trash };
