import { appState, activeSession } from "./state.js";

const messagesEl = document.getElementById("messages");
const welcomeEl = document.getElementById("welcome");
const titleEl = document.getElementById("activeSessionTitle");

export function renderChat() {
  const session = activeSession();
  titleEl.textContent = session ? session.title : "新会话";
  messagesEl.innerHTML = "";
  welcomeEl.hidden = appState.messages.length > 0;
  for (const message of appState.messages) {
    messagesEl.appendChild(messageNode(message));
  }
  scrollToBottom();
}

export function appendUserMessage(content, imageName) {
  appState.messages.push({
    role: "user",
    content,
    imageName,
  });
  renderChat();
}

export function ensureAssistantMessage() {
  const last = appState.messages[appState.messages.length - 1];
  if (!last || last.role !== "assistant" || last.done) {
    appState.messages.push({
      role: "assistant",
      content: "",
      reasoning: "",
      tools: [],
      done: false,
    });
  }
  return appState.messages[appState.messages.length - 1];
}

export function appendAssistantChunk(kind, text) {
  const message = ensureAssistantMessage();
  if (kind === "reasoning") {
    message.reasoning += text;
  } else {
    message.content += text;
  }
  renderChat();
}

export function finishAssistant(content, reasoning) {
  const message = ensureAssistantMessage();
  if (content) message.content = content;
  if (reasoning) message.reasoning = reasoning;
  message.done = true;
  renderChat();
}

export function showAssistantError(message) {
  const assistant = ensureAssistantMessage();
  assistant.content += `\n\n错误: ${message}`;
  assistant.done = true;
  renderChat();
}

function messageNode(message) {
  const block = document.createElement("div");
  block.className = `message-block ${message.role}`;
  const bubble = document.createElement("article");
  bubble.className = "bubble";
  if (message.role === "assistant") {
    const meta = document.createElement("div");
    meta.className = "message-meta";
    meta.textContent = "assistant";
    bubble.appendChild(meta);
    if (message.reasoning) {
      const details = document.createElement("details");
      details.className = "reasoning";
      const summary = document.createElement("summary");
      summary.textContent = "thinking";
      const body = document.createElement("div");
      body.textContent = message.reasoning;
      details.append(summary, body);
      bubble.appendChild(details);
    }
  }
  const content = document.createElement("div");
  content.className = "content";
  content.innerHTML = renderText(message.content || "");
  bubble.appendChild(content);
  if (message.imageName) {
    const image = document.createElement("div");
    image.className = "message-meta";
    image.textContent = `attached image: ${message.imageName}`;
    bubble.appendChild(image);
  }
  block.appendChild(bubble);
  return block;
}

function renderText(text) {
  const escaped = escapeHtml(text);
  return escaped
    .replace(/```([\s\S]*?)```/g, "<pre><code>$1</code></pre>")
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\n/g, "<br>");
}

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function scrollToBottom() {
  messagesEl.scrollTop = messagesEl.scrollHeight;
}
