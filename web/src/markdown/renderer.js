// ============================================================
// Markdown 完整渲染入口：协调 marked + KaTeX + highlight.js + Mermaid
// 提供"完整渲染"与"闭合块增强渲染"两套能力
// ============================================================

import {
  unwrapMathFromCode,
  protectMath,
  restoreMathPlaceholders,
  renderMath,
} from "./math.js";
import {
  renderCodeBlockShells,
  highlightCodeBlocks,
} from "./code.js";
import { renderMermaid } from "./mermaid.js";

// 初始化 marked 选项：gfm 表格 + 换行生效
if (window.marked && typeof window.marked.setOptions === "function") {
  window.marked.setOptions({ gfm: true, breaks: true });
}

/**
 * 渲染已闭合的 Markdown 增强内容：代码块外壳、Mermaid、高亮、公式
 * @param {HTMLElement} container - 待处理的消息容器
 * @param {boolean} skipLast - 流式模式下是否跳过最后未闭合块
 * @returns {void}
 */
export function renderCompletedBlocks(container, skipLast = false) {
  if (!container) return;
  // 1. 代码块外壳（语言标签 + 复制按钮）
  renderCodeBlockShells(container);
  // 2. Mermaid 图表替换
  renderMermaid(container, skipLast);
  // 3. 代码高亮
  highlightCodeBlocks(container, skipLast);
  // 4. 数学公式渲染
  renderMath(container, skipLast);
}

/**
 * 完整渲染一条消息的 Markdown 内容
 * 流程：剥离公式反引号 → 占位保护 → marked.parse → 还原占位 → 增强渲染
 * @param {HTMLElement} container - 消息内容容器
 * @param {string} text - 原始 Markdown 文本
 * @returns {void}
 */
export function renderFull(container, text) {
  if (!container) return;
  const safe = String(text || "");
  // 1. 记录原始文本，供代码块闭合判断使用
  container.setAttribute("data-raw", safe);
  if (!window.marked) {
    // 1.1 marked 未加载时降级为纯文本
    container.textContent = safe;
    return;
  }
  // 2. 剥离公式反引号并占位保护
  const unwrapped = unwrapMathFromCode(safe);
  const { text: protectedText, mathMap } = protectMath(unwrapped);
  // 3. marked 解析
  container.innerHTML = window.marked.parse(protectedText).trim();
  // 4. 还原占位公式
  restoreMathPlaceholders(container, mathMap);
  // 5. 增强渲染闭合块
  renderCompletedBlocks(container, false);
}

/**
 * 仅渲染用户消息的纯文本（转义 + 换行，不走 Markdown）
 * @param {HTMLElement} container - 消息内容容器
 * @param {string} text - 原始文本
 * @returns {void}
 */
export function renderPlainText(container, text) {
  if (!container) return;
  const div = document.createElement("div");
  div.textContent = String(text || "");
  container.innerHTML = div.innerHTML;
  container.setAttribute("data-raw", String(text || ""));
}
