import type { AppConfig, SubagentConfig, SubagentProfileConfig } from "../../../api/contracts";
import { Select } from "../../../shared/ui/select/select";

type SubagentSettingsPanelProps = {
  config: AppConfig;
  onConfigChange: (config: AppConfig) => void;
};

const BUILTIN_PROFILES: SubagentProfileConfig[] = [
  { id: "general", name: "通用 Agent", description: "适合实现、测试、文档和常规工程任务", thinking_level: "auto", exposed: true },
  { id: "explore", name: "探索 Agent", description: "适合只读检索、代码定位和资料探索", thinking_level: "auto", exposed: true }
];

const THINKING_OPTIONS = ["auto", "none", "low", "medium", "high", "xhigh", "max"].map((value) => ({ value, label: value }));

/**
 * 编辑内置与自定义子 Agent 的默认项、暴露范围和模型参数。
 *
 * @param props 应用配置与更新回调
 * @returns 子 Agent 配置面板
 */
export function SubagentSettingsPanel({ config, onConfigChange }: SubagentSettingsPanelProps) {
  const subagent = config.subagent ?? {};
  const stored = subagent.profiles ?? [];
  const profiles = BUILTIN_PROFILES.map((builtin) => ({ ...builtin, ...stored.find((item) => item.id === builtin.id) }));
  const defaultProfile = subagent.default_profile || "general";

  /** 写回子 Agent 配置。 */
  const updateSubagent = (patch: Partial<SubagentConfig>) => {
    onConfigChange({ ...config, subagent: { ...subagent, ...patch } });
  };

  /** 更新指定子 Agent 档案。 */
  const updateProfile = (id: string, patch: Partial<SubagentProfileConfig>) => {
    const next = profiles.map((profile) => profile.id === id ? { ...profile, ...patch } : profile);
    updateSubagent({ profiles: next });
  };

  return (
    <section className="subagent-settings-panel">
      <header><div><h3>子 Agent</h3><p>主 Agent 根据已暴露档案的描述选择子 Agent。</p></div></header>
      <div className="subagent-default-row">
        <label><span>默认子 Agent</span><Select value={defaultProfile} options={profiles.filter((profile) => profile.exposed !== false).map((profile) => ({ value: profile.id, label: profile.name }))} onChange={(value) => updateSubagent({ default_profile: value })} /></label>
      </div>
      <div className="subagent-profile-list">
        {profiles.map((profile) => {
          const provider = config.providers.find((item) => item.id === profile.provider_id);
          const models = provider?.models ?? [];
          return <article key={profile.id} className="subagent-profile-editor">
            <div className="subagent-profile-head"><strong>{profile.name}</strong><label className="settings-switch"><input type="checkbox" checked={profile.exposed !== false} onChange={(event) => updateProfile(profile.id, { exposed: event.target.checked })} /><span /><em>{profile.exposed !== false ? "已暴露" : "未暴露"}</em></label></div>
            <label><span>描述</span><input value={profile.description ?? ""} onChange={(event) => updateProfile(profile.id, { description: event.target.value })} /></label>
            <label><span>系统提示词</span><textarea value={profile.system_prompt ?? ""} onChange={(event) => updateProfile(profile.id, { system_prompt: event.target.value })} placeholder="留空使用内置提示词" /></label>
            <div className="subagent-profile-grid">
              <label><span>供应商</span><Select value={profile.provider_id || ""} options={[{ value: "", label: "沿用主 Agent" }, ...config.providers.map((item) => ({ value: item.id, label: item.display_name || item.id }))]} onChange={(value) => updateProfile(profile.id, { provider_id: value, model: "" })} /></label>
              <label><span>模型</span><Select value={profile.model || ""} options={[{ value: "", label: "供应商默认" }, ...models.map((model) => ({ value: model, label: model }))]} onChange={(value) => updateProfile(profile.id, { model: value })} disabled={!profile.provider_id} /></label>
              <label><span>思考等级</span><Select value={profile.thinking_level || "auto"} options={THINKING_OPTIONS} onChange={(value) => updateProfile(profile.id, { thinking_level: value })} /></label>
            </div>
          </article>;
        })}
      </div>
    </section>
  );
}
