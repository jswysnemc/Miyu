import { appState } from "./state.js";
import { appendUserMessage } from "./chat-view.js";
import { sendCurrentMessage } from "./stream.js";

const formEl = document.getElementById("composer");
const inputEl = document.getElementById("messageInput");
const imageInputEl = document.getElementById("imageInput");
const attachmentStripEl = document.getElementById("attachmentStrip");

export function setupComposer() {
  inputEl.addEventListener("input", resizeInput);
  inputEl.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      formEl.requestSubmit();
    }
  });
  imageInputEl.addEventListener("change", handleImageSelected);
  formEl.addEventListener("submit", async (event) => {
    event.preventDefault();
    const message = inputEl.value.trim();
    if ((!message && !appState.pendingImageUrl) || appState.streaming) return;
    const imageUrl = appState.pendingImageUrl;
    const imageName = appState.pendingImageName;
    appendUserMessage(message, imageName);
    inputEl.value = "";
    clearAttachment();
    resizeInput();
    await sendCurrentMessage(message, imageUrl);
  });
}

function handleImageSelected() {
  const file = imageInputEl.files && imageInputEl.files[0];
  if (!file) return;
  const reader = new FileReader();
  reader.onload = () => {
    appState.pendingImageUrl = String(reader.result || "");
    appState.pendingImageName = file.name;
    attachmentStripEl.hidden = false;
    attachmentStripEl.textContent = `已选择图片: ${file.name}`;
  };
  reader.readAsDataURL(file);
}

function clearAttachment() {
  appState.pendingImageUrl = null;
  appState.pendingImageName = null;
  imageInputEl.value = "";
  attachmentStripEl.hidden = true;
  attachmentStripEl.textContent = "";
}

function resizeInput() {
  inputEl.style.height = "auto";
  inputEl.style.height = `${Math.min(inputEl.scrollHeight, 180)}px`;
}
