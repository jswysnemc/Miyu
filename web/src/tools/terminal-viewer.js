// ============================================================
// 终端风格卡片渲染器：为命令行工具与后台任务执行提供高质感终端卡片
// 支持终端顶栏、执行命令回显、输出文本限制与等宽格式化
// ============================================================

import { icon } from "../icons.js";

/**
 * 渲染终端卡片视图
 * @param {HTMLElement} container - 挂载容器
 * @param {string} commandText - 执行的命令行
 * @param {string} outputText - 终端输出文本
 * @param {boolean} [isSuccess=true] - 退出或执行状态是否成功
 * @returns {void}
 */
export function renderTerminalView(container, commandText, outputText, isSuccess = true) {
  container.innerHTML = "";
  
  const termBox = document.createElement("div");
  termBox.className = `terminal-viewer-box ${isSuccess ? "term-ok" : "term-err"}`;

  // 1. 终端卡片标头栏
  const header = document.createElement("div");
  header.className = "terminal-header";
  header.innerHTML = `
    <span class="term-title">
      <span class="term-icon">${icon("terminal")}</span>
      <strong>Codex Terminal</strong>
    </span>
    <span class="term-status ${isSuccess ? "status-ok" : "status-err"}">${isSuccess ? "0 (SUCCESS)" : "ERR"}</span>
  `;
  termBox.appendChild(header);

  // 2. 命令行执行记录
  if (commandText) {
    const cmdBar = document.createElement("div");
    cmdBar.className = "terminal-cmd-bar";
    cmdBar.innerHTML = `<span class="term-prompt">$</span> <span class="term-cmd-text">${escapeHtml(commandText)}</span>`;
    termBox.appendChild(cmdBar);
  }

  // 3. 终端回显区
  const body = document.createElement("pre");
  body.className = "terminal-body";
  const cleanOutput = (outputText || "").trim() || (isSuccess ? "（无输出结果）" : "（执行失败或返回为空）");
  body.textContent = cleanOutput;
  termBox.appendChild(body);

  container.appendChild(termBox);
}

/**
 * HTML 转义工具函数
 * @param {string} str - 原始字符串
 * @returns {string} 转义结果
 */
function escapeHtml(str) {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}
