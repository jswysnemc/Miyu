// ============================================================
// 入口：初始化各模块、绑定全局事件、注入图标
// ============================================================

import { apiGet } from "./api.js";
import { icons } from "./icons.js";
import { initTheme, toggleTheme } from "./theme.js";
import { setupScroll } from "./chat/view.js";
import { setupComposer } from "./composer/input.js";
import { setupAttachments } from "./composer/attachments.js";
import { setupStop } from "./composer/controls.js";
import { loadSessions, createSession } from "./sessions/list.js";
import { setupSessionSearch } from "./sessions/search.js";
import { setupSessionManage } from "./sessions/manage.js";
import { setupInspector } from "./inspector/drawer.js";
import { loadGateways } from "./inspector/gateways.js";
import { loadTasks } from "./inspector/tasks.js";
import {
  loadConfig,
  openSettings,
  closeSettings,
  saveConfig,
  setupSettingsTabs,
} from "./settings/modal.js";
import { setupLightbox } from "./overlays/lightbox.js";
import { setupMermaidViewer } from "./overlays/mermaid-viewer.js";
import { abortCurrentStream } from "./stream.js";
import { createCustomSelect } from "./components/custom-select.js";

/**
 * 初始化应用
 * @returns {Promise<void>}
 */
async function init() {
  // 1. 注入自研选框与图标、主题
  initCustomSelects();
  injectIcons();
  initTheme();
  // 2. 健康检查
  await apiGet("/api/health");
  // 3. 初始化各模块
  setupScroll();
  setupComposer();
  setupAttachments();
  setupSessionSearch();
  setupSessionManage();
  setupInspector();
  setupSettingsTabs();
  setupLightbox();
  setupMermaidViewer();
  setupStop(abortCurrentStream);
  // 4. 绑定全局动作
  bindActions();
  // 5. 并行加载初始数据
  await Promise.all([loadConfig(), loadSessions(), loadGateways(), loadTasks()]);
}

/**
 * 为所有带 data-icon 的元素注入对应 SVG
 * @returns {void}
 */
function injectIcons() {
  document.querySelectorAll("[data-icon]").forEach((node) => {
    const name = node.dataset.icon;
    if (icons[name]) node.innerHTML = icons[name];
  });
}

/**
 * 绑定全局按钮动作
 * @returns {void}
 */
function bindActions() {
  // 1. 会话
  document.getElementById("newSessionBtn").addEventListener("click", createSession);
  // 2. 侧边栏
  document.getElementById("sidebarToggleBtn").addEventListener("click", toggleSidebar);
  document.getElementById("sidebarHideBtn").addEventListener("click", toggleSidebar);
  document.getElementById("sidebarOverlay").addEventListener("click", closeMobileSidebar);
  // 3. 主题
  document.getElementById("themeToggleBtn").addEventListener("click", toggleTheme);
  // 4. 设置
  document.getElementById("settingsBtn").addEventListener("click", openSettings);
  document.getElementById("closeSettingsBtn").addEventListener("click", closeSettings);
  document.getElementById("reloadConfigBtn").addEventListener("click", loadConfig);
  document.getElementById("saveConfigBtn").addEventListener("click", saveConfig);
  // 5. 检查器刷新
  document.getElementById("refreshGatewaysBtn").addEventListener("click", loadGateways);
  document.getElementById("refreshTasksBtn").addEventListener("click", loadTasks);
}

/**
 * 切换侧边栏：桌面切换 collapsed，移动切换 open + overlay
 * @returns {void}
 */
function toggleSidebar() {
  const sidebar = document.getElementById("sidebar");
  const overlay = document.getElementById("sidebarOverlay");
  const isMobile = window.matchMedia("(max-width: 900px)").matches;
  if (isMobile) {
    const open = sidebar.classList.toggle("open");
    overlay.classList.toggle("show", open);
  } else {
    sidebar.classList.toggle("collapsed");
  }
}

/**
 * 关闭移动端侧边栏
 * @returns {void}
 */
function closeMobileSidebar() {
  document.getElementById("sidebar").classList.remove("open");
  document.getElementById("sidebarOverlay").classList.remove("show");
}

/**
 * 初始化并挂载定制下拉选择器（取替原生 HTML select）
 * @returns {void}
 */
function initCustomSelects() {
  const modeWrap = document.getElementById("modeSelectWrap");
  if (modeWrap) {
    const modeSelect = createCustomSelect({
      options: [
        { value: "yolo", label: "YOLO (全自动)" },
        { value: "plan", label: "PLAN (需确认)" },
      ],
      value: "yolo",
      ariaLabel: "智能执行模式",
    });
    modeSelect.id = "modeSelect";
    modeWrap.appendChild(modeSelect);
  }

  const thinkingWrap = document.getElementById("thinkingSelectWrap");
  if (thinkingWrap) {
    const thinkingSelect = createCustomSelect({
      options: [
        { value: "", label: "思考: 自动" },
        { value: "none", label: "关闭思考" },
        { value: "low", label: "轻度思考" },
        { value: "medium", label: "中度思考" },
        { value: "high", label: "深度思考" },
        { value: "xhigh", label: "极深推理" },
        { value: "max", label: "最大算力" },
      ],
      value: "",
      ariaLabel: "推理算力等级",
    });
    thinkingSelect.id = "thinkingSelect";
    thinkingWrap.appendChild(thinkingSelect);
  }
}

init().catch((error) => {
  const stateEl = document.getElementById("connectionState");
  if (stateEl) stateEl.textContent = error.message;
  console.info("【前端/初始化】启动失败", error);
});
