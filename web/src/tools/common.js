// ============================================================
// 工具渲染通用能力：图标集、卡片骨架、参数解析、输出区渲染
// 各具体工具渲染器复用这里的构造函数与样式类名
// ============================================================

import { escapeHtml, truncate } from "../dom.js";
import { renderFull } from "../markdown/renderer.js";

/**
 * 工具专用图标集，键名与工具类别对应
 * @type {Record<string, string>}
 */
export const toolIcons = {
  file: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>',
  edit: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"/></svg>',
  trash: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>',
  search: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>',
  terminal: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>',
  globe: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>',
  image: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/></svg>',
  brain: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9.5 2A2.5 2.5 0 0 1 12 4.5v15a2.5 2.5 0 0 1-4.96.44 2.5 2.5 0 0 1-2.96-3.08 3 3 0 0 1-.34-5.58 2.5 2.5 0 0 1 1.32-4.24 2.5 2.5 0 0 1 1.98-3A2.5 2.5 0 0 1 9.5 2Z"/><path d="M14.5 2A2.5 2.5 0 0 0 12 4.5v15a2.5 2.5 0 0 0 4.96.44 2.5 2.5 0 0 0 2.96-3.08 3 3 0 0 0 .34-5.58 2.5 2.5 0 0 0-1.32-4.24 2.5 2.5 0 0 0-1.98-3A2.5 2.5 0 0 0 14.5 2Z"/></svg>',
  book: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/></svg>',
  sparkles: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M5.6 18.4l2.1-2.1M16.3 7.7l2.1-2.1"/></svg>',
  clock: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>',
  cloud: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.5 19a4.5 4.5 0 1 0 0-9h-1.8A7 7 0 1 0 4 15.3"/></svg>',
  coins: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="8" r="6"/><path d="M18.09 10.37A6 6 0 1 1 10.34 18"/><path d="M7 6h1v4M16.71 13.88l.7.71-2.82 2.82"/></svg>',
  download: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>',
  rocket: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4.5 16.5c-1.5 1.26-2 5-2 5s3.74-.5 5-2c.71-.84.7-2.13-.09-2.91a2.18 2.18 0 0 0-2.91-.09z"/><path d="M12 15l-3-3a22 22 0 0 1 2-3.95A12.88 12.88 0 0 1 22 2c0 2.72-.78 7.5-6 11a22.35 22.35 0 0 1-4 2z"/><path d="M9 12H4s.55-3.03 2-4c1.62-1.08 5 0 5 0M12 15v5s3.03-.55 4-2c1.08-1.62 0-5 0-5"/></svg>',
  layers: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/></svg>',
  dice: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="3"/><circle cx="8" cy="8" r="1.2" fill="currentColor"/><circle cx="16" cy="16" r="1.2" fill="currentColor"/><circle cx="16" cy="8" r="1.2" fill="currentColor"/><circle cx="8" cy="16" r="1.2" fill="currentColor"/></svg>',
  plug: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 2v6M15 2v6M5 8h14v3a7 7 0 0 1-14 0V8zM12 18v4"/></svg>',
  package: '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="16.5" y1="9.4" x2="7.5" y2="4.21"/><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>',
};

/**
 * 安全解析工具参数 JSON 字符串
 * @param {string} raw - 参数 JSON 文本
 * @returns {Object} 解析后的对象，失败返回空对象
 */
export function parseArgs(raw) {
  if (!raw) return {};
  try {
    return typeof raw === "string" ? JSON.parse(raw) : raw;
  } catch {
    return {};
  }
}

/**
 * 构造工具调用卡片骨架
 * @param {Object} opts - 选项
 * @param {string} opts.name - 工具名
 * @param {string} opts.iconSvg - 图标 SVG
 * @param {string} opts.summary - 摘要文本
 * @param {boolean} [opts.running] - 是否运行中
 * @returns {{block: HTMLElement, barEl: HTMLElement, summaryEl: HTMLElement, bodyEl: HTMLElement, statusEl: HTMLElement}}
 */
export function createToolCard({ name, iconSvg, summary, running = true }) {
  const block = document.createElement("div");
  block.className = "tool-call";
  block.dataset.tool = name;
  block.dataset.status = running ? "running" : "ok";

  // 1. 标题栏：图标 + 工具名 + 摘要 + 状态点 + 折叠箭头
  const bar = document.createElement("div");
  bar.className = "tool-call-bar";

  const iconWrap = document.createElement("span");
  iconWrap.className = "tool-call-icon";
  iconWrap.innerHTML = iconSvg || toolIcons.package;

  const nameEl = document.createElement("span");
  nameEl.className = "tool-call-name";
  nameEl.textContent = name;

  const summaryEl = document.createElement("span");
  summaryEl.className = "tool-call-summary";
  summaryEl.textContent = summary || "";

  const statusEl = document.createElement("span");
  statusEl.className = "tool-call-status";

  const chevron = document.createElement("span");
  chevron.className = "tool-call-chevron";
  chevron.innerHTML =
    '<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"/></svg>';

  bar.append(iconWrap, nameEl, summaryEl, statusEl, chevron);
  block.appendChild(bar);

  // 2. 输出区：默认展开（运行中），完成后折叠
  const body = document.createElement("div");
  body.className = "tool-call-body";
  block.appendChild(body);

  // 3. 点击标题栏切换折叠
  bar.addEventListener("click", () => {
    block.classList.toggle("collapsed");
  });

  return { block, barEl: bar, summaryEl, bodyEl: body, statusEl };
}

/**
 * 设置卡片状态
 * @param {HTMLElement} block - 卡片根元素
 * @param {HTMLElement} statusEl - 状态点元素
 * @param {"running"|"ok"|"error"} status - 状态
 * @param {boolean} autoCollapse - 完成后是否自动折叠
 * @returns {void}
 */
export function setCardStatus(block, statusEl, status, autoCollapse = true) {
  block.dataset.status = status;
  if (status === "running") {
    statusEl.className = "tool-call-status running";
    block.classList.remove("collapsed");
  } else if (status === "ok") {
    statusEl.className = "tool-call-status ok";
    if (autoCollapse) block.classList.add("collapsed");
  } else {
    statusEl.className = "tool-call-status error";
    // 错误保持展开
    block.classList.remove("collapsed");
  }
}

/**
 * 在输出区渲染纯文本输出，带行号风格
 * @param {HTMLElement} bodyEl - 输出区元素
 * @param {string} output - 输出文本
 * @param {number} [max] - 最大字符数
 * @returns {void}
 */
export function renderTextOutput(bodyEl, output, max = 4000) {
  const text = truncate(output, max);
  const pre = document.createElement("pre");
  pre.className = "tool-output-text";
  pre.textContent = text;
  bodyEl.innerHTML = "";
  bodyEl.appendChild(pre);
}

/**
 * 在输出区渲染 Markdown 输出（用于报告类工具）
 * @param {HTMLElement} bodyEl - 输出区元素
 * @param {string} output - 输出文本
 * @param {number} [max] - 最大字符数
 * @returns {void}
 */
export function renderMarkdownOutput(bodyEl, output, max = 8000) {
  const text = truncate(output, max);
  const wrapper = document.createElement("div");
  wrapper.className = "tool-output-md";
  bodyEl.innerHTML = "";
  bodyEl.appendChild(wrapper);
  renderFull(wrapper, text);
}

/**
 * 在输出区渲染错误信息
 * @param {HTMLElement} bodyEl - 输出区元素
 * @param {string} message - 错误文本
 * @returns {void}
 */
export function renderErrorOutput(bodyEl, message) {
  const err = document.createElement("div");
  err.className = "tool-output-error";
  err.textContent = String(message || "工具执行失败");
  bodyEl.innerHTML = "";
  bodyEl.appendChild(err);
}

/**
 * 转义文本并截断的便捷组合，用于摘要
 * @param {string} text - 原始文本
 * @param {number} max - 最大长度
 * @returns {string}
 */
export function clipSummary(text, max = 80) {
  const value = String(text || "").replace(/\s+/g, " ").trim();
  return value.length > max ? value.slice(0, max) + "…" : value;
}
