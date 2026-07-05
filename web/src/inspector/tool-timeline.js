// ============================================================
// 工具调用时间线：收集、渲染、清空
// ============================================================

import { appState } from "../state.js";
import { escapeHtml, truncate } from "../dom.js";

const timelineEl = document.getElementById("toolTimeline");

/**
 * 添加一条工具事件并刷新时间线
 * @param {Object} event - 工具事件 {type, name, output, message, arguments_preview, arguments, ok}
 * @returns {void}
 */
export function addToolEvent(event) {
  appState.toolEvents.unshift({
    ...event,
    time: new Date().toLocaleTimeString(),
  });
  // 1. 仅保留最近 80 条
  appState.toolEvents = appState.toolEvents.slice(0, 80);
  renderToolTimeline();
}

/**
 * 清空工具时间线
 * @returns {void}
 */
export function clearToolTimeline() {
  appState.toolEvents = [];
  renderToolTimeline();
}

/**
 * 渲染工具时间线
 * @returns {void}
 */
export function renderToolTimeline() {
  if (appState.toolEvents.length === 0) {
    timelineEl.innerHTML = '<div class="empty-note">暂无工具调用</div>';
    return;
  }
  timelineEl.innerHTML = appState.toolEvents.map(renderToolItem).join("");
}

/**
 * 渲染单个工具事件
 * @param {Object} event - 工具事件
 * @returns {string} HTML 字符串
 */
function renderToolItem(event) {
  const status = event.type === "tool_result" ? (event.ok ? "ok" : "error") : "running";
  const name = event.name || `tool-${event.index ?? ""}`;
  const body = event.output || event.message || event.arguments_preview || event.arguments || "";
  return `
    <div class="tool-item">
      <div class="tool-head">
        <span class="tool-name">${escapeHtml(name)}</span>
        <span class="status-pill ${status}">${escapeHtml(event.type)}</span>
      </div>
      <div class="tool-body">${escapeHtml(truncate(body, 1200))}</div>
      <div class="tool-time">${escapeHtml(event.time || "")}</div>
    </div>
  `;
}
