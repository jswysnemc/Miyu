// ============================================================
// 图像类工具渲染器：analyze_image/print_image/generate_image
// 摘要显示图片路径或提示词，输出区尝试渲染图片预览
// ============================================================

import {
  toolIcons,
  parseArgs,
  renderTextOutput,
  renderErrorOutput,
  clipSummary,
} from "./common.js";

/**
 * 从输出文本中尝试提取图片 URL 或路径
 * @param {string} output - 工具输出
 * @returns {string|null}
 */
function extractImageUrl(output) {
  if (!output) return null;
  // 1. 优先匹配 http(s) 图片链接
  const match = String(output).match(/https?:\/\/\S+\.(?:png|jpe?g|webp|gif)/i);
  if (match) return match[0];
  // 2. 匹配本地路径（/开头或包含 Pictures/miyu）
  const local = String(output).match(/(\/[^\s]+\.(?:png|jpe?g|webp|gif))/i);
  if (local) return local[1];
  return null;
}

/**
 * 在输出区渲染图片预览
 * @param {HTMLElement} bodyEl - 输出区元素
 * @param {string} output - 工具输出
 * @returns {void}
 */
function renderImagePreview(bodyEl, output) {
  const url = extractImageUrl(output);
  if (url) {
    const img = document.createElement("img");
    img.className = "tool-output-image";
    img.src = url;
    img.alt = "";
    img.addEventListener("click", () => {
      window.dispatchEvent(new CustomEvent("miyu:lightbox", { detail: { src: url } }));
    });
    bodyEl.innerHTML = "";
    bodyEl.appendChild(img);
    return;
  }
  // 1. 无图片链接时降级为文本
  renderTextOutput(bodyEl, output, 3000);
}

const analyze = {
  icon: toolIcons.image,
  /**
   * 摘要：显示分析目标
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const p = args.path || args.image_path || args.url || "";
    return p ? `分析图片 ${clipSummary(p, 80)}` : "分析图片";
  },
  /**
   * 渲染输出：分析结果文本
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 6000);
  },
};

const print = {
  icon: toolIcons.image,
  /**
   * 摘要：显示图片路径
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const p = args.path || args.image_path || args.url || "";
    return p ? `展示图片 ${clipSummary(p, 80)}` : "展示图片";
  },
  /**
   * 渲染输出：图片预览
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderImagePreview(ctx.bodyEl, ctx.output);
  },
};

const generate = {
  icon: toolIcons.sparkles,
  /**
   * 摘要：显示生成提示词
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const p = args.prompt || args.description || "";
    return p ? `生成图片 "${clipSummary(p, 70)}"` : "生成图片";
  },
  /**
   * 渲染输出：生成结果图片预览
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderImagePreview(ctx.bodyEl, ctx.output);
  },
};

export default { analyze, print, generate };
