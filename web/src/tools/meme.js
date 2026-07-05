// ============================================================
// 表情包类工具渲染器：search_meme/show_meme/add_meme
// 摘要显示关键词，输出区尝试渲染表情包图片预览
// ============================================================

import {
  toolIcons,
  parseArgs,
  renderTextOutput,
  renderErrorOutput,
  clipSummary,
} from "./common.js";

/**
 * 从输出中提取图片路径或 URL
 * @param {string} output - 工具输出
 * @returns {string|null}
 */
function extractMemeImage(output) {
  if (!output) return null;
  const match = String(output).match(/(https?:\/\/\S+\.(?:png|jpe?g|webp|gif)|\/[^\s]+\.(?:png|jpe?g|webp|gif))/i);
  return match ? match[1] : null;
}

/**
 * 渲染表情包图片预览
 * @param {HTMLElement} bodyEl - 输出区元素
 * @param {string} output - 工具输出
 * @returns {void}
 */
function renderMemePreview(bodyEl, output) {
  const url = extractMemeImage(output);
  if (url) {
    const img = document.createElement("img");
    img.className = "tool-output-image tool-output-meme";
    img.src = url;
    img.alt = "";
    img.addEventListener("click", () => {
      window.dispatchEvent(new CustomEvent("miyu:lightbox", { detail: { src: url } }));
    });
    bodyEl.innerHTML = "";
    bodyEl.appendChild(img);
    return;
  }
  renderTextOutput(bodyEl, output, 2000);
}

const search = {
  icon: toolIcons.image,
  /**
   * 摘要：显示搜索关键词
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const q = args.query || args.keyword || args.tag || "";
    return q ? `搜表情 "${clipSummary(q, 60)}"` : "搜索表情包";
  },
  /**
   * 渲染输出
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderMemePreview(ctx.bodyEl, ctx.output);
  },
};

const show = {
  icon: toolIcons.image,
  /**
   * 摘要：显示表情包名
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const n = args.name || args.id || args.keyword || "";
    return n ? `表情 ${clipSummary(n, 60)}` : "展示表情包";
  },
  /**
   * 渲染输出
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderMemePreview(ctx.bodyEl, ctx.output);
  },
};

const add = {
  icon: toolIcons.download,
  /**
   * 摘要：显示添加来源
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const n = args.name || args.url || args.source || "";
    return n ? `添加表情 ${clipSummary(n, 60)}` : "添加表情包";
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

export default { search, show, add };
