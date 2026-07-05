// ============================================================
// Mermaid 全屏查看器：监听 miyu:mermaid-zoom 事件
// 支持滚轮缩放、拖拽平移、重置
// ============================================================

const viewer = document.getElementById("mermaidViewer");
const canvas = document.getElementById("mermaidViewerCanvas");
const body = document.getElementById("mermaidViewerBody");

let scale = 1;
let translateX = 0;
let translateY = 0;
let dragging = false;
let dragStartX = 0;
let dragStartY = 0;

/**
 * 初始化 Mermaid 查看器事件
 * @returns {void}
 */
export function setupMermaidViewer() {
  // 1. 按钮图标
  document.getElementById("mermaidZoomInBtn").innerHTML =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><line x1="11" y1="8" x2="11" y2="14"/><line x1="8" y1="11" x2="14" y2="11"/></svg>';
  document.getElementById("mermaidZoomOutBtn").innerHTML =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><line x1="8" y1="11" x2="14" y2="11"/></svg>';

  // 2. 监听图表放大请求
  window.addEventListener("miyu:mermaid-zoom", (event) => {
    if (event.detail && event.detail.svg) open(event.detail.svg);
  });

  document.getElementById("mermaidCloseBtn").addEventListener("click", close);
  document.getElementById("mermaidZoomInBtn").addEventListener("click", () => zoom(1.2));
  document.getElementById("mermaidZoomOutBtn").addEventListener("click", () => zoom(0.8));
  document.getElementById("mermaidResetBtn").addEventListener("click", reset);

  // 3. 滚轮缩放
  body.addEventListener("wheel", (event) => {
    event.preventDefault();
    zoom(event.deltaY < 0 ? 1.1 : 0.9);
  });

  // 4. 拖拽平移
  body.addEventListener("mousedown", startDrag);
  document.addEventListener("mousemove", onDrag);
  document.addEventListener("mouseup", endDrag);

  // 5. Esc 关闭
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && !viewer.hidden) close();
  });
}

/**
 * 打开查看器
 * @param {string} svgHtml - SVG 字符串
 * @returns {void}
 */
function open(svgHtml) {
  canvas.innerHTML = svgHtml;
  scale = 1;
  translateX = 0;
  translateY = 0;
  applyTransform();
  viewer.hidden = false;
}

/**
 * 关闭查看器
 * @returns {void}
 */
function close() {
  viewer.hidden = true;
  canvas.innerHTML = "";
}

/**
 * 缩放
 * @param {number} factor - 缩放系数
 * @returns {void}
 */
function zoom(factor) {
  scale = Math.max(0.2, Math.min(5, scale * factor));
  applyTransform();
}

/**
 * 重置缩放与平移
 * @returns {void}
 */
function reset() {
  scale = 1;
  translateX = 0;
  translateY = 0;
  applyTransform();
}

/**
 * 应用变换到画布
 * @returns {void}
 */
function applyTransform() {
  canvas.style.transform = `translate(${translateX}px, ${translateY}px) scale(${scale})`;
}

/**
 * 开始拖拽
 * @param {MouseEvent} event - 鼠标事件
 * @returns {void}
 */
function startDrag(event) {
  if (viewer.hidden) return;
  dragging = true;
  dragStartX = event.clientX - translateX;
  dragStartY = event.clientY - translateY;
  body.classList.add("dragging");
}

/**
 * 拖拽中
 * @param {MouseEvent} event - 鼠标事件
 * @returns {void}
 */
function onDrag(event) {
  if (!dragging) return;
  translateX = event.clientX - dragStartX;
  translateY = event.clientY - dragStartY;
  applyTransform();
}

/**
 * 结束拖拽
 * @returns {void}
 */
function endDrag() {
  dragging = false;
  body.classList.remove("dragging");
}
