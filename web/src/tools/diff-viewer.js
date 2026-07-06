// ============================================================
// 差异对比（Diff）渲染器：为文件修改与代码编辑工具提供高对比视图
// 支持行号显示、增加/删除红绿样式高亮与代码块结构布局
// ============================================================

import { icon } from "../icons.js";

/**
 * 判断文本是否符合 Diff 语法特征
 * @param {string} text - 待检测输出文本
 * @returns {boolean} 是否为 Diff
 */
export function isDiffText(text) {
  if (!text || typeof text !== "string") return false;
  const lines = text.trim().split("\n");
  if (lines.length < 2) return false;
  // 1. 检查特征标头
  return lines.some((line) => line.startsWith("--- ") || line.startsWith("+++ ") || line.startsWith("@@ ") || line.startsWith("<<<<<<< ") || line.startsWith(">>>>>>> "));
}

/**
 * 渲染 Diff 差异对比视图
 * @param {HTMLElement} container - 目标挂载容器
 * @param {string} diffText - 差异文本
 * @param {string} [title="差异比对视图"] - 视图标题
 * @returns {void}
 */
export function renderDiffView(container, diffText, title = "文件更改差异比对") {
  container.innerHTML = "";
  
  const diffBox = document.createElement("div");
  diffBox.className = "diff-viewer-box";

  // 1. 顶部标题栏
  const header = document.createElement("div");
  header.className = "diff-viewer-header";
  header.innerHTML = `
    <span class="diff-title-left">
      <span class="diff-icon">${icon("diff")}</span>
      <strong>${title}</strong>
    </span>
    <span class="diff-badge">Codex Diff</span>
  `;
  diffBox.appendChild(header);

  // 2. 行比对内容主体
  const contentEl = document.createElement("div");
  contentEl.className = "diff-viewer-body";

  const lines = (diffText || "").split("\n");
  let oldLineNum = 1;
  let newLineNum = 1;

  lines.forEach((line) => {
    const rowEl = document.createElement("div");
    rowEl.className = "diff-line-row";

    let lineType = "normal";
    let oldNumText = String(oldLineNum);
    let newNumText = String(newLineNum);

    if (line.startsWith("--- ") || line.startsWith("+++ ") || line.startsWith("Index:") || line.startsWith("====")) {
      lineType = "meta";
      oldNumText = "";
      newNumText = "";
    } else if (line.startsWith("@@ ")) {
      lineType = "hunk";
      oldNumText = "...";
      newNumText = "...";
      // 解析 hunk 标头行号，比如 @@ -10,5 +10,6 @@
      const match = line.match(/@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
      if (match) {
        oldLineNum = Number.parseInt(match[1], 10);
        newLineNum = Number.parseInt(match[2], 10);
      }
    } else if (line.startsWith("+")) {
      lineType = "add";
      oldNumText = "";
      newNumText = String(newLineNum++);
    } else if (line.startsWith("-")) {
      lineType = "remove";
      oldNumText = String(oldLineNum++);
      newNumText = "";
    } else {
      oldLineNum++;
      newLineNum++;
    }

    rowEl.classList.add(`diff-${lineType}`);
    
    rowEl.innerHTML = `
      <span class="diff-line-num old-num">${oldNumText}</span>
      <span class="diff-line-num new-num">${newNumText}</span>
      <span class="diff-line-code">${escapeHtml(line)}</span>
    `;

    contentEl.appendChild(rowEl);
  });

  diffBox.appendChild(contentEl);
  container.appendChild(diffBox);
}

/**
 * 内部 HTML 转义函数
 * @param {string} str - 原始字符串
 * @returns {string} 转义后字符串
 */
function escapeHtml(str) {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}
