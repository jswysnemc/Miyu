// ============================================================
// 检查器抽屉：折叠控制
// ============================================================

import { appState, setInspectorOpen } from "../state.js";

const inspector = document.getElementById("inspector");
const toggleBtn = document.getElementById("inspectorToggleBtn");
const closeBtn = document.getElementById("inspectorCloseBtn");

/**
 * 初始化抽屉开关事件
 * @returns {void}
 */
export function setupInspector() {
  toggleBtn.addEventListener("click", () => {
    toggleInspector(!appState.inspectorOpen);
  });
  closeBtn.addEventListener("click", closeInspector);
}

/**
 * 切换抽屉开关状态
 * @param {boolean} open - 是否展开
 * @returns {void}
 */
export function toggleInspector(open) {
  inspector.classList.toggle("open", open);
  setInspectorOpen(open);
}

/**
 * 打开抽屉
 * @returns {void}
 */
export function openInspector() {
  toggleInspector(true);
}

/**
 * 关闭抽屉
 * @returns {void}
 */
export function closeInspector() {
  toggleInspector(false);
}
