// ============================================================
// DOM 工具：元素创建、转义、防抖、轻提示
// ============================================================

/**
 * 创建 DOM 元素并批量设置属性与子节点
 * @param {string} tag - 标签名
 * @param {Object} [props] - 属性对象，className/class、dataset、on 等会特殊处理
 * @param {Array} [children] - 子节点数组，字符串会被包装为文本节点
 * @returns {HTMLElement} 构造好的元素
 */
export function el(tag, props = {}, children = []) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(props)) {
    if (value == null || value === false) continue;
    if (key === "className" || key === "class") {
      node.className = value;
    } else if (key === "dataset") {
      Object.assign(node.dataset, value);
    } else if (key === "style" && typeof value === "object") {
      Object.assign(node.style, value);
    } else if (key.startsWith("on") && typeof value === "function") {
      node.addEventListener(key.slice(2).toLowerCase(), value);
    } else if (key === "html") {
      node.innerHTML = value;
    } else if (key in node) {
      node[key] = value;
    } else {
      node.setAttribute(key, value);
    }
  }
  for (const child of [].concat(children)) {
    if (child == null || child === false) continue;
    node.appendChild(typeof child === "string" ? document.createTextNode(child) : child);
  }
  return node;
}

/**
 * 转义 HTML 特殊字符，防止注入
 * @param {string} str - 原始文本
 * @returns {string} 转义后的文本
 */
export function escapeHtml(str) {
  const div = document.createElement("div");
  div.textContent = str == null ? "" : String(str);
  return div.innerHTML;
}

/**
 * 防抖：延迟执行，期间再次调用则重置计时
 * @param {Function} fn - 需要防抖的函数
 * @param {number} delay - 延迟毫秒
 * @returns {Function} 防抖后的函数
 */
export function debounce(fn, delay = 120) {
  let timer = null;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), delay);
  };
}

// ============================================================
// 轻提示 Toast
// ============================================================

let toastTimer = null;

/**
 * 显示一条轻提示，2 秒后自动消失
 * @param {string} message - 提示文本
 * @returns {void}
 */
export function showToast(message) {
  let toast = document.getElementById("toast");
  if (!toast) {
    toast = el("div", { id: "toast", className: "toast" });
    document.body.appendChild(toast);
  }
  toast.textContent = message;
  toast.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toast.classList.remove("show"), 2000);
}

/**
 * 将文本裁剪到指定长度并追加省略号
 * @param {string} text - 原始文本
 * @param {number} max - 最大长度
 * @returns {string} 裁剪后的文本
 */
export function truncate(text, max = 1200) {
  const value = String(text || "");
  return value.length > max ? value.slice(0, max) + "…" : value;
}
