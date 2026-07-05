// ============================================================
// 会话搜索：输入关键词实时过滤
// ============================================================

import { debounce } from "../dom.js";
import { renderSessions } from "./list.js";

const searchInput = document.getElementById("sessionSearchInput");

/**
 * 初始化会话搜索输入监听
 * @returns {void}
 */
export function setupSessionSearch() {
  searchInput.addEventListener(
    "input",
    debounce(() => {
      renderSessions(searchInput.value);
    }, 150)
  );
}

/**
 * 清空搜索框并恢复全量列表
 * @returns {void}
 */
export function clearSessionSearch() {
  searchInput.value = "";
  renderSessions("");
}
