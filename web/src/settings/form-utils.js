// ============================================================
// 设置表单工具：字段生成、取值同步、转义
// ============================================================

import { escapeHtml } from "../dom.js";

export { escapeHtml };

/**
 * 生成文本/数字类输入字段
 * @param {string} path - 配置路径
 * @param {string} label - 标签
 * @param {*} value - 当前值
 * @param {string} [type="text"] - input 类型
 * @param {string} [wide=""] - 是否占满整行 "full"
 * @returns {string} HTML 字符串
 */
export function field(path, label, value, type = "text", wide = "") {
  return `
    <div class="field ${wide}">
      <label>${escapeHtml(label)}</label>
      <input data-config-path="${escapeHtml(path)}" type="${type}" value="${escapeHtml(value ?? "")}">
    </div>
  `;
}

/**
 * 生成复选框字段
 * @param {string} path - 配置路径
 * @param {string} label - 标签
 * @param {boolean} value - 当前值
 * @returns {string} HTML 字符串
 */
export function checkbox(path, label, value) {
  return `
    <div class="field">
      <label>${escapeHtml(label)}</label>
      <input data-config-path="${escapeHtml(path)}" type="checkbox" ${value ? "checked" : ""}>
    </div>
  `;
}

/**
 * 按点分路径设置目标对象的值
 * @param {Object} target - 目标对象
 * @param {string} path - 点分路径，如 providers.0.base_url
 * @param {*} value - 值
 * @returns {void}
 */
export function setByPath(target, path, value) {
  const parts = path.split(".");
  let cursor = target;
  for (const part of parts.slice(0, -1)) {
    cursor = cursor[part];
  }
  cursor[parts.at(-1)] = value;
}

/**
 * 首字母大写
 * @param {string} value - 原始字符串
 * @returns {string} 首字母大写后的字符串
 */
export function capitalize(value) {
  return value.slice(0, 1).toUpperCase() + value.slice(1);
}
