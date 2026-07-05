// ============================================================
// 输入区控件状态：发送/停止切换、连接状态文本
// ============================================================

const sendBtn = document.getElementById("sendBtn");
const connectionStateEl = document.getElementById("connectionState");

/**
 * 设置响应中状态：切换发送/停止按钮样式与连接状态文本
 * @param {boolean} responding - 是否正在响应
 * @returns {void}
 */
export function setResponding(responding) {
  if (responding) {
    sendBtn.classList.add("responding");
    sendBtn.type = "button";
    connectionStateEl.textContent = "生成中";
  } else {
    sendBtn.classList.remove("responding");
    sendBtn.type = "submit";
    connectionStateEl.textContent = "本地服务";
  }
}

/**
 * 绑定停止按钮点击（响应中点击则中断流式）
 * @param {Function} onStop - 停止回调
 * @returns {void}
 */
export function setupStop(onStop) {
  sendBtn.addEventListener("click", (event) => {
    if (sendBtn.classList.contains("responding")) {
      event.preventDefault();
      onStop();
    }
  });
}
