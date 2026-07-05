import { apiGet, apiPost } from "./api.js";
import { appState } from "./state.js";

const gatewayListEl = document.getElementById("gatewayList");
const taskListEl = document.getElementById("taskList");

export async function loadGateways() {
  const data = await apiGet("/api/gateways");
  appState.gateways = data.gateways || [];
  renderGateways();
}

export async function loadTasks() {
  const data = await apiGet("/api/tools/background");
  appState.tasks = data.tasks || [];
  renderTasks();
}

async function startGateway(id) {
  await apiPost(`/api/gateways/${encodeURIComponent(id)}/start`);
  await loadGateways();
  await loadTasks();
}

async function stopGateway(id) {
  await apiPost(`/api/gateways/${encodeURIComponent(id)}/stop`);
  await loadGateways();
  await loadTasks();
}

function renderGateways() {
  if (appState.gateways.length === 0) {
    gatewayListEl.innerHTML = '<div class="empty-note">暂无网关配置</div>';
    return;
  }
  gatewayListEl.innerHTML = appState.gateways.map((gateway) => `
    <div class="gateway-item">
      <div class="gateway-head">
        <span class="gateway-name">${escapeHtml(gateway.title)}</span>
        <span class="status-pill ${gateway.status === "running" ? "running" : ""}">${escapeHtml(gateway.status)}</span>
      </div>
      <div class="tool-body">config=${gateway.enabled ? "enabled" : "disabled"}${gateway.pid ? ` pid=${gateway.pid}` : ""}</div>
      <div class="gateway-actions">
        <button data-start-gateway="${gateway.id}">启动</button>
        <button data-stop-gateway="${gateway.id}">停止</button>
      </div>
    </div>
  `).join("");
  gatewayListEl.querySelectorAll("[data-start-gateway]").forEach((button) => {
    button.addEventListener("click", () => startGateway(button.dataset.startGateway));
  });
  gatewayListEl.querySelectorAll("[data-stop-gateway]").forEach((button) => {
    button.addEventListener("click", () => stopGateway(button.dataset.stopGateway));
  });
}

function renderTasks() {
  if (appState.tasks.length === 0) {
    taskListEl.innerHTML = '<div class="empty-note">暂无后台任务</div>';
    return;
  }
  taskListEl.innerHTML = appState.tasks.slice().reverse().map((task) => `
    <div class="task-item">
      <div class="task-head">
        <span class="task-name">${escapeHtml(task.label || task.command)}</span>
        <span class="status-pill ${task.status === "running" ? "running" : ""}">${escapeHtml(task.status)}</span>
      </div>
      <div class="tool-body">pid=${task.pid} id=${escapeHtml(task.id)}</div>
    </div>
  `).join("");
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}
