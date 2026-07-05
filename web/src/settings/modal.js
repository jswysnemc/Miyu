// ============================================================
// 设置弹窗：加载、开关、保存、标签页切换
// ============================================================

import { apiGet, apiPut } from "../api.js";
import { appState } from "../state.js";
import { renderProviders } from "./providers.js";
import { renderTools } from "./tools.js";
import { renderGateways } from "./gateways.js";
import { setByPath } from "./form-utils.js";

const modalEl = document.getElementById("settingsModal");
const statusEl = document.getElementById("settingsStatus");

/**
 * 加载配置并渲染
 * @returns {Promise<void>}
 */
export async function loadConfig() {
  const data = await apiGet("/api/config");
  appState.config = data.config;
  renderSettings();
}

/**
 * 打开设置弹窗
 * @returns {void}
 */
export function openSettings() {
  renderSettings();
  modalEl.hidden = false;
}

/**
 * 关闭设置弹窗
 * @returns {void}
 */
export function closeSettings() {
  modalEl.hidden = true;
}

/**
 * 保存当前配置
 * @returns {Promise<void>}
 */
export async function saveConfig() {
  syncConfigFromForm();
  statusEl.textContent = "保存中";
  const data = await apiPut("/api/config", { config: appState.config });
  appState.config = data.config;
  statusEl.textContent = "已保存";
  renderSettings();
}

/**
 * 初始化标签页切换
 * @returns {void}
 */
export function setupSettingsTabs() {
  document.querySelectorAll(".tab-button").forEach((button) => {
    button.addEventListener("click", () => {
      document.querySelectorAll(".tab-button").forEach((item) => item.classList.remove("active"));
      document.querySelectorAll(".settings-pane").forEach((item) => item.classList.remove("active"));
      button.classList.add("active");
      const target = document.getElementById(`settings${capitalize(button.dataset.tab)}`);
      if (target) target.classList.add("active");
    });
  });
}

/**
 * 渲染所有配置面板
 * @returns {void}
 */
function renderSettings() {
  if (!appState.config) return;
  renderProviders();
  renderTools();
  renderGateways();
}

/**
 * 从表单输入同步配置回 appState.config
 * @returns {void}
 */
function syncConfigFromForm() {
  document.querySelectorAll("[data-config-path]").forEach((input) => {
    const path = input.dataset.configPath;
    let value = input.type === "checkbox" ? input.checked : input.value;
    // 1. 模型列表按逗号拆分
    if (path.endsWith(".models")) {
      value = input.value
        .split(",")
        .map((item) => item.trim())
        .filter(Boolean);
    } else if (input.type === "number") {
      // 2. 数字解析
      value = input.value.includes(".")
        ? Number.parseFloat(input.value)
        : Number.parseInt(input.value, 10);
      if (Number.isNaN(value)) value = 0;
    }
    setByPath(appState.config, path, value);
  });
  // 3. 同步当前 provider
  const activeProvider = document.getElementById("activeProviderField");
  if (activeProvider) {
    appState.config.active_provider = activeProvider.value;
  }
}

function capitalize(value) {
  return value.slice(0, 1).toUpperCase() + value.slice(1);
}
