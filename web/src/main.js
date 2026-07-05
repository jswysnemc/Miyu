import { apiGet } from "./api.js";
import { setupComposer } from "./composer.js";
import { loadGateways, loadTasks } from "./gateways-view.js";
import { loadSessions, createSession } from "./sessions-view.js";
import {
  closeSettings,
  loadConfig,
  openSettings,
  saveConfig,
  setupSettingsTabs,
} from "./settings-view.js";
import { clearToolTimeline, renderToolTimeline } from "./tool-timeline.js";

async function init() {
  await apiGet("/api/health");
  setupComposer();
  setupSettingsTabs();
  bindActions();
  renderToolTimeline();
  await Promise.all([
    loadConfig(),
    loadSessions(),
    loadGateways(),
    loadTasks(),
  ]);
}

function bindActions() {
  document.getElementById("newSessionBtn").addEventListener("click", createSession);
  document.getElementById("settingsBtn").addEventListener("click", openSettings);
  document.getElementById("closeSettingsBtn").addEventListener("click", closeSettings);
  document.getElementById("reloadConfigBtn").addEventListener("click", loadConfig);
  document.getElementById("saveConfigBtn").addEventListener("click", saveConfig);
  document.getElementById("refreshGatewaysBtn").addEventListener("click", loadGateways);
  document.getElementById("refreshTasksBtn").addEventListener("click", loadTasks);
  document.getElementById("clearToolsBtn").addEventListener("click", clearToolTimeline);
  document.getElementById("sidebarToggle").addEventListener("click", () => {
    document.getElementById("sidebar").classList.toggle("open");
  });
}

init().catch((error) => {
  document.getElementById("connectionState").textContent = error.message;
});
