// ============================================================
// 设置工作区主控视图（兼容导出原有 openSettings 等方法名）
// 实现由常规 Modal 改造为应用内全屏 / 分栏视图，控制子面板路由
// ============================================================

import { apiGet, apiPut } from "../api.js";
import { appState } from "../state.js";
import { renderProviders } from "./providers.js";
import { renderTools } from "./tools.js";
import { renderGateways } from "./gateways.js";
import { setByPath, capitalize, syncFormValues } from "./form-utils.js";
import { showToast } from "../components/toast.js";

const wsEl = document.getElementById("settingsWorkspace");

/**
 * 加载配置并渲染
 * @returns {Promise<void>}
 */
export async function loadConfig() {
  try {
    const data = await apiGet("/api/config");
    appState.config = data.config;
    renderSettings();
    showToast("配置同步完成", "success", 2000);
  } catch (error) {
    showToast(`配置加载失败: ${error.message}`, "error", 4000);
  }
}

/**
 * 打开设置控制台工作区
 * @returns {void}
 */
export function openSettings() {
  renderSettings();
  if (wsEl) {
    wsEl.classList.add("show");
  }
}

/**
 * 关闭设置控制台工作区
 * @returns {void}
 */
export function closeSettings() {
  if (wsEl) {
    wsEl.classList.remove("show");
  }
}

/**
 * 保存当前配置
 * @returns {Promise<void>}
 */
export async function saveConfig() {
  syncConfigFromForm();
  showToast("正在保存设置...", "info", 1500);
  try {
    const data = await apiPut("/api/config", { config: appState.config });
    appState.config = data.config;
    renderSettings();
    showToast("配置已成功保存更新", "success", 3000);
  } catch (error) {
    showToast(`保存配置失败: ${error.message}`, "error", 4000);
  }
}

/**
 * 初始化设置侧边导航分类点击路由
 * @returns {void}
 */
export function setupSettingsTabs() {
  document.querySelectorAll(".settings-nav .nav-item").forEach((button) => {
    button.addEventListener("click", () => {
      document.querySelectorAll(".settings-nav .nav-item").forEach((item) => item.classList.remove("active"));
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
 * 从常规表单项同步基础配置值（自研复杂组件已在组件交互时同步更新 appState.config）
 * @returns {void}
 */
function syncConfigFromForm() {
  syncFormValues(wsEl || document.body, appState.config);
}
