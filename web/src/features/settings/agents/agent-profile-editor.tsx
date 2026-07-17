import { useEffect, useState } from "react";
import { Trash2 } from "lucide-react";
import type { AppConfig } from "../../../api/contracts";
import { Button } from "../../../shared/ui/button/button";
import { TextArea } from "../../../shared/ui/form/text-area";
import type { AgentProfile } from "../../agents/agent-types";
import { DEFAULT_AGENT_ID } from "../../agents/agent-options";
import { EditorHeader } from "../editor-layout";
import type { AgentOptions } from "./agents-types";
import { AgentSkillPermissions } from "./agent-skill-permissions";
import { AgentToolPermissions } from "./agent-tool-permissions";
import { AgentRuntimeFields } from "./agent-runtime-fields";

type AgentEditorTab = "basic" | "tools" | "skills";

type AgentProfileEditorProps = {
  config: AppConfig;
  profile: AgentProfile;
  options: AgentOptions;
  onChange: (patch: Partial<AgentProfile>) => void;
  onRemove: () => void;
};

/**
 * 渲染主 Agent 档案编辑器，并按基础配置、工具权限和 Skills 分页。
 *
 * @param props 当前配置、档案、可选能力和操作回调
 * @returns 主 Agent 档案编辑器
 */
export function AgentProfileEditor({ config, profile, options, onChange, onRemove }: AgentProfileEditorProps) {
  const [tab, setTab] = useState<AgentEditorTab>("basic");

  useEffect(() => {
    setTab("basic");
  }, [profile.id]);

  return (
    <section className="settings-editor agent-profile-editor">
      <EditorHeader
        kicker="Agent"
        title={profile.name || profile.id}
        description={`${profile.id}，已启用 ${profile.enabled_tools.length} 个工具和 ${profile.skills_full.length + profile.skills_named.length} 个 Skills。`}
        actions={<>
          {profile.id !== DEFAULT_AGENT_ID && (
            <label className="settings-switch">
              <input type="checkbox" checked={profile.register_to_main} onChange={(event) => onChange({ register_to_main: event.target.checked })} />
              <span />
              <strong>{profile.register_to_main ? "已向主 Agent 注册" : "未注册"}</strong>
            </label>
          )}
          {profile.id !== DEFAULT_AGENT_ID && !["general", "explore"].includes(profile.id) && (
            <Button className="settings-danger" onClick={onRemove}>
              <Trash2 size={14} />删除档案
            </Button>
          )}
        </>}
      />
      <nav className="settings-tabs agent-editor-tabs" aria-label="Agent 配置分类">
        <Button className={tab === "basic" ? "active" : ""} onClick={() => setTab("basic")}>基础配置</Button>
        <Button className={tab === "tools" ? "active" : ""} onClick={() => setTab("tools")}>工具权限</Button>
        <Button className={tab === "skills" ? "active" : ""} onClick={() => setTab("skills")}>Skills</Button>
      </nav>
      {tab === "basic" && (
        <div className="agent-basic-form">
          <div className="settings-form-grid">
            <label className="settings-field">
              <span>显示名称</span>
              <input value={profile.name} onChange={(event) => onChange({ name: event.target.value })} />
              <small>用于 Agent 选择菜单和运行状态展示</small>
            </label>
            <AgentRuntimeFields
              config={config}
              providerId={profile.provider_id}
              model={profile.model}
              thinkingLevel={profile.thinking_level}
              inheritModelLabel="沿用当前模型"
              thinkingHelp="覆盖供应商的默认推理强度"
              onChange={onChange}
            />
            <label className="settings-field full">
              <span>用途描述</span>
              <input value={profile.description} onChange={(event) => onChange({ description: event.target.value })} />
              <small>主 Agent 根据这段描述判断是否调用该 Agent</small>
            </label>
          </div>
          <label className="settings-field agent-prompt-field">
            <span>系统提示词</span>
            <TextArea
              value={profile.system_prompt}
              onChange={(event) => onChange({ system_prompt: event.target.value })}
              placeholder="描述职责、边界和输出要求"
            />
            <small>只写长期稳定的角色约束，具体任务仍由会话输入提供。</small>
          </label>
        </div>
      )}
      {tab === "tools" && (
        <AgentToolPermissions
          tools={options.tools}
          enabled={profile.enabled_tools}
          onChange={(enabledTools) => onChange({ enabled_tools: enabledTools })}
        />
      )}
      {tab === "skills" && (
        <AgentSkillPermissions
          skills={options.skills}
          fullNames={profile.skills_full}
          namedNames={profile.skills_named}
          onChange={(fullNames, namedNames) => onChange({ skills_full: fullNames, skills_named: namedNames })}
        />
      )}
    </section>
  );
}
