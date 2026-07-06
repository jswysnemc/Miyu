/* ============================================================
 * 设置表单与定制交互组件库 (form-utils)
 * 彻底废除 HTML 原生 checkbox 与原生 select，提供高定滑动开关、
 * 选项芯片及沉浸输入框组件，并处理状态数据同步
 * ============================================================ */

import { escapeHtml } from "../dom.js";

export { escapeHtml };

/**
 * 生成沉浸式文本/数字/密码输入字段
 * @param {string} path - 配置路径
 * @param {string} label - 标签
 * @param {*} value - 当前值
 * @param {string} [type="text"] - input 类型
 * @param {string} [wide=""] - "full" 占满整行
 * @param {string} [desc=""] - 辅助说明或占位提示
 * @returns {string} HTML 字符串
 */
export function field(path, label, value, type = "text", wide = "", desc = "") {
  return `
    <div class="custom-field-wrap ${wide}">
      <div class="field-label-row">
        <span class="field-title">${escapeHtml(label)}</span>
        ${desc ? `<span class="field-hint">${escapeHtml(desc)}</span>` : ""}
      </div>
      <div class="custom-input-container">
        <input class="custom-input-field" data-config-path="${escapeHtml(path)}" type="${type}" value="${escapeHtml(value ?? "")}" placeholder="${escapeHtml(desc || label)}" autocomplete="off" spellcheck="false">
      </div>
    </div>
  `;
}

/**
 * 生成自研美学滑动开关（取替 HTML 原生 checkbox）
 * @param {string} path - 配置路径
 * @param {string} label - 开关名称
 * @param {boolean} value - 当前布尔状态
 * @param {string} [desc=""] - 补充解释说明
 * @param {string} [wide=""] - 是否占宽
 * @returns {string} HTML 字符串
 */
export function checkbox(path, label, value, desc = "", wide = "") {
  const isChecked = !!value;
  return `
    <div class="custom-toggle-wrap ${wide}">
      <div class="toggle-text">
        <span class="toggle-label">${escapeHtml(label)}</span>
        ${desc ? `<span class="toggle-desc">${escapeHtml(desc)}</span>` : ""}
      </div>
      <div class="custom-toggle-btn ${isChecked ? "active" : ""}" data-config-path="${escapeHtml(path)}" data-value="${isChecked ? "true" : "false"}" role="switch" aria-checked="${isChecked ? "true" : "false"}" tabindex="0">
        <div class="toggle-slider"></div>
      </div>
    </div>
  `;
}

/**
 * 生成定制芯片分段选择器（取替 HTML 原生 select 与简陋文本输入）
 * @param {string} path - 配置路径
 * @param {string} label - 标签名
 * @param {Array<{label: string, value: string}>} options - 可选项目表
 * @param {string} currentValue - 当前选中的值
 * @param {string} [desc=""] - 补充说明
 * @param {string} [wide=""] - 占宽控制
 * @returns {string} HTML 字符串
 */
export function choicePills(path, label, options, currentValue, desc = "", wide = "") {
  return `
    <div class="custom-field-wrap ${wide}">
      <div class="field-label-row">
        <span class="field-title">${escapeHtml(label)}</span>
        ${desc ? `<span class="field-hint">${escapeHtml(desc)}</span>` : ""}
      </div>
      <div class="custom-choice-group" data-config-path="${escapeHtml(path)}" data-value="${escapeHtml(currentValue ?? "")}">
        ${options.map((opt) => `
          <div class="choice-pill ${opt.value === currentValue ? "selected" : ""}" data-val="${escapeHtml(opt.value)}">${escapeHtml(opt.label)}</div>
        `).join("")}
      </div>
    </div>
  `;
}

/**
 * 为表单容器绑定自研开关与芯片分段的点击委托事件
 * @param {HTMLElement} container - 面板或表单根容器
 * @returns {void}
 */
export function setupFormInteractions(container) {
  if (!container || container._hasSetupInteractions) return;
  container._hasSetupInteractions = true;

  container.addEventListener("click", (e) => {
    // 1. 处理自定义滑动开关（Toggle Button）
    const toggleBtn = e.target.closest(".custom-toggle-btn");
    if (toggleBtn && container.contains(toggleBtn)) {
      const currentVal = toggleBtn.dataset.value === "true";
      const newVal = !currentVal;
      toggleBtn.dataset.value = newVal ? "true" : "false";
      toggleBtn.setAttribute("aria-checked", newVal ? "true" : "false");
      toggleBtn.classList.toggle("active", newVal);
      return;
    }

    // 2. 处理芯片分段选择器（Choice Pill）
    const pill = e.target.closest(".choice-pill");
    if (pill && container.contains(pill)) {
      const group = pill.closest(".custom-choice-group");
      if (group) {
        group.querySelectorAll(".choice-pill").forEach((p) => p.classList.remove("selected"));
        pill.classList.add("selected");
        group.dataset.value = pill.dataset.val || "";
      }
    }
  });

  // 3. 支持键盘空格/回车切换 Toggle
  container.addEventListener("keydown", (e) => {
    if (e.key === " " || e.key === "Enter") {
      const toggleBtn = e.target.closest(".custom-toggle-btn");
      if (toggleBtn && container.contains(toggleBtn)) {
        e.preventDefault();
        toggleBtn.click();
      }
    }
  });
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
 * 从容器内部的所有配置项安全收集并写入应用配置
 * @param {HTMLElement} container - 表单 DOM 根节点
 * @param {Object} configTarget - 接收设置修改的全局配置对象
 * @returns {void}
 */
export function syncFormValues(container, configTarget) {
  if (!container || !configTarget) return;

  // 1. 处理自定义文本/数字字段
  container.querySelectorAll("input.custom-input-field[data-config-path]").forEach((input) => {
    const path = input.dataset.configPath;
    let value = input.value;
    if (input.type === "number") {
      value = input.value.includes(".")
        ? Number.parseFloat(input.value)
        : Number.parseInt(input.value, 10);
      if (Number.isNaN(value)) value = 0;
    }
    setByPath(configTarget, path, value);
  });

  // 2. 处理自定义滑动开关
  container.querySelectorAll(".custom-toggle-btn[data-config-path]").forEach((toggle) => {
    const path = toggle.dataset.configPath;
    const value = toggle.dataset.value === "true";
    setByPath(configTarget, path, value);
  });

  // 3. 处理定制芯片分段选择
  container.querySelectorAll(".custom-choice-group[data-config-path]").forEach((group) => {
    const path = group.dataset.configPath;
    const value = group.dataset.value ?? "";
    setByPath(configTarget, path, value);
  });
}

/**
 * 首字母大写
 * @param {string} value - 原始字符串
 * @returns {string} 首字母大写后的字符串
 */
export function capitalize(value) {
  return value.slice(0, 1).toUpperCase() + value.slice(1);
}
