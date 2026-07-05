// ============================================================
// 网关管理：列表、启动、停止
// ============================================================

import { apiGet, apiPost } from "../api.js";
import { appState } from "../state.js";
import { escapeHtml } from "../dom.js";
import { loadTasks } from "./tasks.js";

const gatewayListEl = document.getElementById("gatewayList");

/**
 * 加载网关列表
 * @returns {Promise<void>}
 */
export async function loadGateways() {
  const data = await apiGet("/api/gateways");
  appState.gateways = data.gateways || [];
  renderGateways();
}

/**
 * 启动指定网关
 * @param {string} id - 网关 ID
 * @returns {Promise<void>}
 */
async function startGateway(id) {
  await apiPost(`/api/gateways/${encodeURIComponent(id)}/start`);
  await loadGateways();
  await loadTasks();
}

/**
 * 停止指定网关
 * @param {string} id - 网关 ID
 * @returns {Promise<void>}
 */
async function stopGateway(id) {
  await apiPost(`/api/gateways/${encodeURIComponent(id)}/stop`);
  await loadGateways();
  await loadTasks();
}

/**
 * 渲染网关列表
 * @returns {void}
 */
function renderGateways() {
  if (appState.gateways.length === 0) {
    gatewayListEl.innerHTML = '<div class="empty-note">暂无网关配置</div>';
    return;
  }
  gatewayListEl.innerHTML = appState.gateways
    .map(
      (gateway) => `
    <div class="gateway-item">
      <div class="gateway-head">
        <span class="gateway-name">${escapeHtml(gateway.title)}</span>
        <span class="status-pill ${gateway.status === "running" ? "running" : ""}">${escapeHtml(gateway.status)}</span>
      </div>
      <div class="gateway-meta">config=${gateway.enabled ? "enabled" : "disabled"}${gateway.pid ? ` pid=${gateway.pid}` : ""}</div>
      <div class="gateway-actions">
        <button data-start="${escapeHtml(gateway.id)}">启动</button>
        <button data-stop="${escapeHtml(gateway.id)}">停止</button>
      </div>
    </div>
  `
    )
    .join("");
  // 1. 绑定启动/停止按钮
  gatewayListEl.querySelectorAll("[data-start]").forEach((btn) => {
    btn.addEventListener("click", () => startGateway(btn.dataset.start));
  });
  gatewayListEl.querySelectorAll("[data-stop]").forEach((btn) => {
    btn.addEventListener("click", () => stopGateway(btn.dataset.stop));
  });
}
