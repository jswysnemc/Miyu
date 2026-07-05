// ============================================================
// 消息操作栏：复制（后续可扩展重试、编辑等）
// ============================================================

import { icons } from "../icons.js";
import { showToast } from "../dom.js";

/**
 * 构造消息操作栏
 * @param {string} role - 消息角色 user/assistant
 * @param {HTMLElement} contentEl - 消息内容元素，用于提取原始文本
 * @returns {HTMLElement} 操作栏元素
 */
export function createMessageActions(role, contentEl) {
  const actions = document.createElement("div");
  actions.className = "message-actions";

  // 1. 复制按钮：读取 data-raw 原始 Markdown，回退到 textContent
  const copyBtn = document.createElement("button");
  copyBtn.className = "action-icon";
  copyBtn.title = "复制";
  copyBtn.innerHTML = icons.copy;
  copyBtn.addEventListener("click", () => {
    const text = contentEl.getAttribute("data-raw") || contentEl.textContent || "";
    navigator.clipboard
      .writeText(text)
      .then(() => {
        copyBtn.innerHTML = icons.check;
        setTimeout(() => {
          copyBtn.innerHTML = icons.copy;
        }, 1500);
      })
      .catch(() => showToast("复制失败"));
  });
  actions.appendChild(copyBtn);

  return actions;
}
