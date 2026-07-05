// ============================================================
// 图片灯箱：监听 miyu:lightbox 事件，放大显示图片
// ============================================================

const lightbox = document.getElementById("imgLightbox");
const imgEl = document.getElementById("imgLightboxImg");
const closeBtn = document.getElementById("lightboxCloseBtn");

/**
 * 初始化灯箱事件监听
 * @returns {void}
 */
export function setupLightbox() {
  // 1. 监听全局图片放大请求
  window.addEventListener("miyu:lightbox", (event) => {
    if (event.detail && event.detail.src) openLightbox(event.detail.src);
  });
  closeBtn.addEventListener("click", closeLightbox);
  // 2. 点击背景关闭
  lightbox.addEventListener("click", (event) => {
    if (event.target === lightbox) closeLightbox();
  });
  // 3. Esc 关闭
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && !lightbox.hidden) closeLightbox();
  });
}

/**
 * 打开灯箱
 * @param {string} src - 图片地址
 * @returns {void}
 */
function openLightbox(src) {
  imgEl.src = src;
  lightbox.hidden = false;
}

/**
 * 关闭灯箱
 * @returns {void}
 */
function closeLightbox() {
  lightbox.hidden = true;
  imgEl.src = "";
}
