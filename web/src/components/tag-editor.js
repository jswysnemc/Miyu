// ============================================================
// 标签芯片编辑器组件：替代粗糙的逗号分隔字符串输入框
// 支持回车与逗号分隔转换标签、单击移除图标或退格键删除
// ============================================================

import { icon } from "../icons.js";

/**
 * 创建标签编辑器节点
 * @param {Object} config - 配置参数 { tags: Array<string>, onChange: Function, placeholder: string }
 * @returns {HTMLElement} 标签编辑器节点
 */
export function createTagEditor({ tags = [], onChange, placeholder = "输入模型名称并按 Enter 添加..." }) {
  const container = document.createElement("div");
  container.className = "tag-editor-container";

  let currentTags = Array.isArray(tags) ? [...tags] : [];

  const chipsWrap = document.createElement("div");
  chipsWrap.className = "tag-chips-wrap";

  const inputEl = document.createElement("input");
  inputEl.type = "text";
  inputEl.className = "tag-editor-input";
  inputEl.placeholder = currentTags.length === 0 ? placeholder : "";

  // 1. 触发值变更回调
  const notifyChange = () => {
    inputEl.placeholder = currentTags.length === 0 ? placeholder : "";
    if (typeof onChange === "function") {
      onChange(currentTags);
    }
  };

  // 2. 渲染当前标签阵列
  const renderChips = () => {
    chipsWrap.innerHTML = "";
    currentTags.forEach((tagText, index) => {
      const chip = document.createElement("span");
      chip.className = "tag-chip";
      chip.innerHTML = `
        <span class="chip-label">${tagText}</span>
        <button type="button" class="chip-remove" title="移除">${icon("close")}</button>
      `;
      chip.querySelector(".chip-remove").addEventListener("click", (event) => {
        event.stopPropagation();
        currentTags.splice(index, 1);
        renderChips();
        notifyChange();
      });
      chipsWrap.appendChild(chip);
    });
    chipsWrap.appendChild(inputEl);
  };

  // 3. 监听键盘事件进行添加与退格删除
  inputEl.addEventListener("keydown", (event) => {
    if (event.key === "Enter" || event.key === ",") {
      event.preventDefault();
      const val = inputEl.value.trim();
      if (val && !currentTags.includes(val)) {
        currentTags.push(val);
        inputEl.value = "";
        renderChips();
        notifyChange();
      }
    } else if (event.key === "Backspace" && inputEl.value === "" && currentTags.length > 0) {
      currentTags.pop();
      renderChips();
      notifyChange();
    }
  });

  inputEl.addEventListener("blur", () => {
    const val = inputEl.value.trim();
    if (val && !currentTags.includes(val)) {
      currentTags.push(val);
      inputEl.value = "";
      renderChips();
      notifyChange();
    }
  });

  container.addEventListener("click", () => {
    inputEl.focus();
  });

  renderChips();
  container.appendChild(chipsWrap);

  // 4. 暴露获取与设置方法
  container.getTags = () => [...currentTags];
  container.setTags = (newTags) => {
    currentTags = Array.isArray(newTags) ? [...newTags] : [];
    renderChips();
    notifyChange();
  };

  return container;
}
