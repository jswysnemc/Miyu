// ============================================================
// 会话管理：多选模式、批量删除
// ============================================================

import { appState } from "../state.js";
import { renderSessions, updateMultiSelectCount, deleteSession } from "./list.js";
import { showToast } from "../dom.js";

const multiSelectBar = document.getElementById("multiSelectBar");
const toggleBtn = document.getElementById("toggleMultiSelectBtn");
const selectAllBtn = document.getElementById("msSelectAllBtn");
const cancelBtn = document.getElementById("msCancelBtn");
const deleteBtn = document.getElementById("msDeleteBtn");

/**
 * 初始化会话管理事件
 * @returns {void}
 */
export function setupSessionManage() {
  toggleBtn.addEventListener("click", toggleMultiSelect);
  selectAllBtn.addEventListener("click", selectAll);
  cancelBtn.addEventListener("click", cancelMultiSelect);
  deleteBtn.addEventListener("click", batchDelete);
}

/**
 * 切换多选模式
 * @returns {void}
 */
export function toggleMultiSelect() {
  appState.multiSelectMode = !appState.multiSelectMode;
  if (!appState.multiSelectMode) {
    appState.selectedIds.clear();
  }
  multiSelectBar.classList.toggle("show", appState.multiSelectMode);
  updateMultiSelectCount();
  renderSessions();
}

/**
 * 全选当前会话
 * @returns {void}
 */
function selectAll() {
  appState.selectedIds = new Set(appState.sessions.map((s) => s.id));
  renderSessions();
  updateMultiSelectCount();
}

/**
 * 取消多选模式
 * @returns {void}
 */
function cancelMultiSelect() {
  appState.multiSelectMode = false;
  appState.selectedIds.clear();
  multiSelectBar.classList.remove("show");
  renderSessions();
}

/**
 * 批量删除选中的会话
 * @returns {Promise<void>}
 */
async function batchDelete() {
  if (appState.selectedIds.size === 0) {
    showToast("未选择会话");
    return;
  }
  const ids = Array.from(appState.selectedIds);
  // 1. 逐个删除，遇到失败继续
  for (const id of ids) {
    try {
      await deleteSession(id);
    } catch (err) {
      console.info("【前端/会话】删除失败", id, err);
    }
  }
  appState.selectedIds.clear();
  appState.multiSelectMode = false;
  multiSelectBar.classList.remove("show");
  renderSessions();
  showToast(`已删除 ${ids.length} 个会话`);
}
