// ============================================================
// 后台任务：列表渲染
// ============================================================

import { apiGet } from "../api.js";
import { appState } from "../state.js";
import { escapeHtml } from "../dom.js";

const taskListEl = document.getElementById("taskList");

/**
 * 加载后台任务列表
 * @returns {Promise<void>}
 */
export async function loadTasks() {
  const data = await apiGet("/api/tools/background");
  appState.tasks = data.tasks || [];
  renderTasks();
}

/**
 * 渲染后台任务列表
 * @returns {void}
 */
function renderTasks() {
  if (appState.tasks.length === 0) {
    taskListEl.innerHTML = '<div class="empty-note">暂无后台任务</div>';
    return;
  }
  // 1. 倒序展示，最新任务在前
  taskListEl.innerHTML = appState.tasks
    .slice()
    .reverse()
    .map(
      (task) => `
    <div class="task-item">
      <div class="task-head">
        <span class="task-name">${escapeHtml(task.label || task.command)}</span>
        <span class="status-pill ${task.status === "running" ? "running" : ""}">${escapeHtml(task.status)}</span>
      </div>
      <div class="task-meta">pid=${escapeHtml(String(task.pid))} id=${escapeHtml(task.id)}</div>
    </div>
  `
    )
    .join("");
}
