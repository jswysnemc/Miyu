import { RefreshCw } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import type { AppConfig } from "../../../api/contracts";
import { Button } from "../../../shared/ui/button/button";
import { AgentProfileWorkspace } from "./agent-profile-workspace";
import { fetchAgentOptions } from "./agents-api";
import type { AgentOptions } from "./agents-types";
import { SubagentSettingsPanel } from "./subagent-settings-panel";
import "./agent-settings-layout.css";
import "./agent-profile-form.css";
import "./subagent-settings.css";

type AgentSettingsSectionProps = {
  config: AppConfig;
  onConfigChange: (config: AppConfig) => void;
};

type AgentSettingsView = "profiles" | "subagents";

/**
 * 渲染主 Agent 与子 Agent 两个独立配置工作区。
 *
 * @param props 应用配置和更新回调
 * @returns Agent 设置区域
 */
export function AgentSettingsSection({ config, onConfigChange }: AgentSettingsSectionProps) {
  const [view, setView] = useState<AgentSettingsView>("profiles");
  const [options, setOptions] = useState<AgentOptions>({ tools: [], skills: [] });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  /**
   * 读取主 Agent 可用的工具与 Skills，并同步加载状态。
   *
   * @returns 加载流程完成后的 Promise
  */
  const loadOptions = useCallback(async () => {
    // 1. 重置上一次加载状态
    setLoading(true);
    setError("");
    // 2. 请求能力选项并记录可展示的错误信息
    try {
      setOptions(await fetchAgentOptions());
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadOptions();
  }, [loadOptions]);

  return (
    <section className="agent-settings-shell">
      <nav className="agent-settings-view-tabs" aria-label="Agent 设置范围">
        <Button className={view === "profiles" ? "active" : ""} onClick={() => setView("profiles")}>
          <strong>主 Agent</strong>
          <span>档案、模型与能力权限</span>
        </Button>
        <Button className={view === "subagents" ? "active" : ""} onClick={() => setView("subagents")}>
          <strong>子 Agent</strong>
          <span>委派档案与运行覆盖</span>
        </Button>
      </nav>
      {view === "profiles" && loading && (
        <div className="agent-settings-loading" aria-live="polite">
          <span />
          <div><strong>正在读取 Agent 能力</strong><small>加载工具和 Skills 列表</small></div>
        </div>
      )}
      {view === "profiles" && !loading && error && (
        <div className="agent-settings-load-error">
          <div><strong>Agent 能力加载失败</strong><small>{error}</small></div>
          <Button className="settings-secondary" onClick={() => void loadOptions()}>
            <RefreshCw size={14} />重新加载
          </Button>
        </div>
      )}
      {view === "profiles" && !loading && !error && (
        <AgentProfileWorkspace config={config} options={options} onConfigChange={onConfigChange} />
      )}
      {view === "subagents" && (
        <SubagentSettingsPanel config={config} onConfigChange={onConfigChange} />
      )}
    </section>
  );
}
