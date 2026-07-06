// ============================================================
// 全局通知提示条组件：替代原生 alert 与 confirm 弹窗
// 提供轻量浮动消息支持，并在指定时延后自动销毁
// ============================================================

import { icon } from "../icons.js";

let toastContainerEl = null;

/**
 * 确保全局通知容器已挂载到 DOM 中
 * @returns {HTMLElement} 容器节点
 */
function ensureToastContainer() {
  if (!toastContainerEl) {
    toastContainerEl = document.createElement("div");
    toastContainerEl.className = "toast-container";
    document.body.appendChild(toastContainerEl);
  }
  return toastContainerEl;
}

/**
 * 触发全局浮动消息提示
 * @param {string} message - 提示文字
 * @param {string} [type="info"] - 消息类型：info / success / error / warn
 * @param {number} [duration=3000] - 自动关闭毫秒数
 * @returns {void}
 */
export function showToast(message, type = "info", duration = 3000) {
  const container = ensureToastContainer();
  // 1. 构建提示条结构
  const item = document.createElement("div");
  item.className = `toast-item toast-${type}`;
  
  let iconName = "info";
  if (type === "success") iconName = "check";
  if (type === "error" || type === "warn") iconName = "close";

  item.innerHTML = `
    <span class="toast-icon">${icon(iconName)}</span>
    <span class="toast-text">${message}</span>
  `;

  container.appendChild(item);
  
  // 2. 触发进场动画
  requestAnimationFrame(() => {
    item.classList.add("show");
  });

  // 3. 定时移除与销毁
  setTimeout(() => {
    item.classList.remove("show");
    setTimeout(() => {
      if (item.parentNode === container) {
        container.removeChild(item);
      }
    }, 250);
  }, duration);
}
