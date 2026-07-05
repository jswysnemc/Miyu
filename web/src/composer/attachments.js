// ============================================================
// 附件：图片上传预览、清除
// ============================================================

import { appState } from "../state.js";
import { icons } from "../icons.js";

const imageInputEl = document.getElementById("imageInput");
const attachmentStripEl = document.getElementById("attachmentStrip");

/**
 * 初始化附件上传事件
 * @returns {void}
 */
export function setupAttachments() {
  imageInputEl.addEventListener("change", handleImageSelected);
}

/**
 * 处理图片选择：读取为 data URL 并显示预览
 * @returns {void}
 */
function handleImageSelected() {
  const file = imageInputEl.files && imageInputEl.files[0];
  if (!file) return;
  const reader = new FileReader();
  reader.onload = () => {
    appState.pendingImageUrl = String(reader.result || "");
    appState.pendingImageName = file.name;
    renderAttachmentChip(appState.pendingImageUrl, file.name);
  };
  reader.readAsDataURL(file);
}

/**
 * 渲染附件预览芯片
 * @param {string} dataUrl - 图片 data URL
 * @param {string} name - 文件名
 * @returns {void}
 */
function renderAttachmentChip(dataUrl, name) {
  attachmentStripEl.hidden = false;
  attachmentStripEl.innerHTML = "";
  const chip = document.createElement("div");
  chip.className = "attachment-chip";

  const img = document.createElement("img");
  img.src = dataUrl;
  img.alt = "";

  const label = document.createElement("span");
  label.textContent = name;
  label.style.overflow = "hidden";
  label.style.textOverflow = "ellipsis";
  label.style.whiteSpace = "nowrap";
  label.style.maxWidth = "160px";

  const remove = document.createElement("span");
  remove.className = "chip-remove";
  remove.innerHTML = icons.close;
  remove.addEventListener("click", clearAttachment);

  chip.append(img, label, remove);
  attachmentStripEl.appendChild(chip);
}

/**
 * 获取当前附件 data URL
 * @returns {string|null}
 */
export function getAttachment() {
  return appState.pendingImageUrl;
}

/**
 * 清除当前附件
 * @returns {void}
 */
export function clearAttachment() {
  appState.pendingImageUrl = null;
  appState.pendingImageName = null;
  imageInputEl.value = "";
  attachmentStripEl.hidden = true;
  attachmentStripEl.innerHTML = "";
}
