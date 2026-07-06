// ============================================================
// 自定义下拉单选组件：彻底替代 HTML 原生 <select>
// 支持点击展开浮层、选项列表浏览、选中状态高亮与事件触发
// ============================================================

import { icon } from "../icons.js";

/**
 * 创建自定义下拉选择器节点
 * @param {Object} config - 配置参数 { options: Array<{value, label}>, value: string, onChange: Function, ariaLabel: string }
 * @returns {HTMLElement} 自定义下拉选择器节点
 */
export function createCustomSelect({ options = [], value = "", onChange, ariaLabel = "选择项" }) {
  const container = document.createElement("div");
  container.className = "custom-select";
  container.setAttribute("role", "combobox");
  container.setAttribute("aria-expanded", "false");
  container.setAttribute("aria-label", ariaLabel);

  let currentValue = value;
  let isOpen = false;

  // 1. 获取当前显示标签
  const getSelectedLabel = () => {
    const target = options.find((opt) => opt.value === currentValue);
    return target ? target.label : currentValue || "请选择";
  };

  // 2. 渲染基础结构
  const trigger = document.createElement("button");
  trigger.type = "button";
  trigger.className = "custom-select-trigger";
  trigger.innerHTML = `
    <span class="select-value">${getSelectedLabel()}</span>
    <span class="select-arrow">${icon("chevronDown")}</span>
  `;

  const dropdown = document.createElement("div");
  dropdown.className = "custom-select-dropdown";
  dropdown.hidden = true;

  // 3. 选项更新方法
  const renderOptions = () => {
    dropdown.innerHTML = "";
    options.forEach((opt) => {
      const item = document.createElement("div");
      item.className = "custom-select-option";
      if (opt.value === currentValue) item.classList.add("selected");
      item.setAttribute("role", "option");
      item.textContent = opt.label || opt.value;
      
      item.addEventListener("click", (event) => {
        event.stopPropagation();
        currentValue = opt.value;
        trigger.querySelector(".select-value").textContent = getSelectedLabel();
        closeDropdown();
        if (typeof onChange === "function") {
          onChange(currentValue);
        }
      });
      dropdown.appendChild(item);
    });
  };

  // 4. 展开与关闭控制
  const openDropdown = () => {
    if (isOpen) return;
    isOpen = true;
    dropdown.hidden = false;
    container.classList.add("open");
    container.setAttribute("aria-expanded", "true");
    renderOptions();
    document.addEventListener("click", handleOutsideClick);
  };

  const closeDropdown = () => {
    if (!isOpen) return;
    isOpen = false;
    dropdown.hidden = true;
    container.classList.remove("open");
    container.setAttribute("aria-expanded", "false");
    document.removeEventListener("click", handleOutsideClick);
  };

  const handleOutsideClick = (event) => {
    if (!container.contains(event.target)) {
      closeDropdown();
    }
  };

  trigger.addEventListener("click", (event) => {
    event.stopPropagation();
    if (isOpen) {
      closeDropdown();
    } else {
      openDropdown();
    }
  });

  container.appendChild(trigger);
  container.appendChild(dropdown);

  // 5. 暴露外部更新当前值方法
  container.setValue = (newValue) => {
    currentValue = newValue;
    const valueSpan = trigger.querySelector(".select-value");
    if (valueSpan) valueSpan.textContent = getSelectedLabel();
  };

  container.getValue = () => currentValue;

  Object.defineProperty(container, "value", {
    get: () => currentValue,
    set: (val) => container.setValue(val),
  });

  return container;
}
