import type { AppConfig } from "../../../api/contracts";
import { Select } from "../../../shared/ui/select/select";
import { buildVisibleAgentProfiles } from "./agent-profile-state";
import type { AgentOptions } from "./agents-types";

type AgentSurfaceDefaultsProps = {
  config: AppConfig;
  options: AgentOptions;
  onConfigChange: (config: AppConfig) => void;
};

/**
 * 配置 Web、TUI 和单次 CLI 默认使用的 Agent。
 *
 * @param props 应用配置、Agent 能力选项和更新回调
 * @returns 三个运行入口的默认 Agent 选择器
 */
export function AgentSurfaceDefaults({ config, options, onConfigChange }: AgentSurfaceDefaultsProps) {
  const profiles = buildVisibleAgentProfiles(config.agents, options, config.subagent?.profiles);
  const choices = profiles.map((profile) => ({ value: profile.id, label: profile.name || profile.id }));

  /**
   * 更新指定入口的默认 Agent。
   *
   * @param field 配置字段
   * @param value Agent 标识
   * @returns 无
   */
  const update = (field: "default_agent" | "tui_agent" | "cli_agent", value: string) => {
    onConfigChange({ ...config, [field]: value === "default" ? null : value });
  };

  return (
    <section className="agent-surface-defaults">
      <div className="settings-section-heading">
        <div><strong>入口默认 Agent</strong><small>分别控制网页工作台、TUI REPL 和单次 CLI 命令。</small></div>
      </div>
      <div className="settings-form-grid">
        <div className="settings-field">
          <span>Web</span>
          <Select value={config.default_agent ?? "default"} options={choices} onChange={(value) => update("default_agent", value)} ariaLabel="Web 默认 Agent" />
        </div>
        <div className="settings-field">
          <span>TUI</span>
          <Select value={config.tui_agent ?? "default"} options={choices} onChange={(value) => update("tui_agent", value)} ariaLabel="TUI 默认 Agent" />
        </div>
        <div className="settings-field">
          <span>CLI</span>
          <Select value={config.cli_agent ?? "default"} options={choices} onChange={(value) => update("cli_agent", value)} ariaLabel="CLI 默认 Agent" />
        </div>
      </div>
    </section>
  );
}
