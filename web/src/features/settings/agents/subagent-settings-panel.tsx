import { useEffect, useMemo, useState } from "react";
import { Bot, Trash2 } from "lucide-react";
import type { AppConfig, SubagentConfig, SubagentProfileConfig } from "../../../api/contracts";
import { Button } from "../../../shared/ui/button/button";
import { useConfirm } from "../../../shared/ui/dialog/dialog-provider";
import { TextArea } from "../../../shared/ui/form/text-area";
import { Select } from "../../../shared/ui/select/select";
import { EditorHeader } from "../editor-layout";
import { ObjectListPanel } from "../object-list-panel";
import { AgentRuntimeFields } from "./agent-runtime-fields";
import {
  BUILTIN_SUBAGENT_PROFILES,
  createUniqueCustomSubagentProfile,
  mergeSubagentProfiles,
  removeCustomSubagentProfile,
  updateSubagentProfile
} from "./subagent-profile-state";

type SubagentSettingsPanelProps = {
  config: AppConfig;
  onConfigChange: (config: AppConfig) => void;
};

const BUILTIN_IDS = new Set(BUILTIN_SUBAGENT_PROFILES.map((profile) => profile.id));

/**
 * 编辑内置与自定义子 Agent 的默认项、暴露范围和模型参数。
 *
 * @param props 应用配置与更新回调
 * @returns 子 Agent 配置工作区
 */
export function SubagentSettingsPanel({ config, onConfigChange }: SubagentSettingsPanelProps) {
  const confirm = useConfirm();
  const subagent = config.subagent ?? {};
  const profiles = useMemo(() => mergeSubagentProfiles(subagent.profiles), [subagent.profiles]);
  const defaultProfile = subagent.default_profile || "general";
  const [selectedId, setSelectedId] = useState(defaultProfile);
  const selected = profiles.find((profile) => profile.id === selectedId) ?? profiles[0] ?? null;
  const visibleProfiles = profiles.filter((profile) => profile.exposed !== false);

  useEffect(() => {
    if (selected && selected.id !== selectedId) setSelectedId(selected.id);
  }, [selected, selectedId]);

  /**
   * 写回子 Agent 配置。
   *
   * @param patch 需要覆盖的子 Agent 配置字段
   * @returns 无返回值
   */
  const updateSubagent = (patch: Partial<SubagentConfig>) => {
    onConfigChange({ ...config, subagent: { ...subagent, ...patch } });
  };

  /**
   * 写回完整子 Agent 档案列表。
   *
   * @param next 更新后的子 Agent 档案数组
   * @param patch 需要同时覆盖的子 Agent 配置字段
   * @returns 无返回值
   */
  const persistProfiles = (next: SubagentProfileConfig[], patch: Partial<SubagentConfig> = {}) => {
    updateSubagent({ ...patch, profiles: next });
  };

  /**
   * 新增并选中自定义子 Agent。
   *
   * @returns 无返回值
  */
  const addProfile = () => {
    // 1. 生成不与内置项和自定义项冲突的新档案
    const created = createUniqueCustomSubagentProfile(profiles);
    // 2. 写回配置并切换到新档案
    persistProfiles([...profiles, created]);
    setSelectedId(created.id);
  };

  /**
   * 更新当前子 Agent，并在隐藏默认项时切换默认档案。
   *
   * @param patch 需要覆盖的子 Agent 档案字段
   * @returns 无返回值
   */
  const updateSelected = (patch: Partial<SubagentProfileConfig>) => {
    if (!selected) return;
    // 1. 更新当前档案并计算可用的默认回退项
    const next = updateSubagentProfile(profiles, selected.id, patch);
    const nextSelected = next.find((profile) => profile.id === selected.id);
    const fallback = next.find((profile) => profile.exposed !== false)?.id ?? "";
    // 2. 当前默认项被隐藏时同步切换默认档案
    persistProfiles(next, {
      default_profile: nextSelected?.exposed === false && defaultProfile === selected.id
        ? fallback
        : defaultProfile
    });
  };

  /**
   * 确认并删除当前自定义子 Agent。
   *
   * @returns 删除流程完成后的 Promise
   */
  const removeSelected = async () => {
    if (!selected || BUILTIN_IDS.has(selected.id)) return;
    // 1. 使用统一确认对话框核对删除操作
    const confirmed = await confirm({
      title: "删除子 Agent",
      description: `将删除“${selected.name || selected.id}”的全部配置。`,
      confirmLabel: "删除子 Agent",
      danger: true
    });
    if (!confirmed) return;
    // 2. 删除自定义档案并修正默认档案
    const next = removeCustomSubagentProfile(profiles, selected.id);
    const fallback = next.find((profile) => profile.exposed !== false)?.id ?? next[0]?.id ?? "";
    persistProfiles(next, { default_profile: defaultProfile === selected.id ? fallback : defaultProfile });
    setSelectedId(fallback);
  };

  const defaultOptions = visibleProfiles.length > 0
    ? visibleProfiles.map((profile) => ({ value: profile.id, label: profile.name }))
    : [{ value: "", label: "没有已暴露的子 Agent" }];

  return (
    <div className="subagent-settings">
      <div className="subagent-default-setting">
        <div>
          <strong>默认子 Agent</strong>
          <small>主 Agent 未指定档案时使用</small>
        </div>
        <Select
          value={visibleProfiles.some((profile) => profile.id === defaultProfile) ? defaultProfile : ""}
          options={defaultOptions}
          onChange={(value) => updateSubagent({ default_profile: value })}
          disabled={visibleProfiles.length === 0}
          ariaLabel="默认子 Agent"
        />
      </div>
      <div className="settings-objects-layout subagent-settings-workspace">
        <ObjectListPanel
          title="子 Agent"
          items={profiles.map((profile) => ({
            id: profile.id,
            name: profile.name || profile.id,
            meta: profile.description || (profile.exposed === false ? "未暴露" : "已暴露"),
            icon: <Bot size={14} />,
            marked: profile.id === defaultProfile
          }))}
          selectedId={selected?.id ?? ""}
          searchPlaceholder="搜索子 Agent"
          addLabel="新增子 Agent"
          onSelect={setSelectedId}
          onAdd={addProfile}
        />
        {selected ? (
          <section className="settings-editor subagent-editor">
            <EditorHeader
              kicker={BUILTIN_IDS.has(selected.id) ? "内置子 Agent" : "自定义子 Agent"}
              title={selected.name || selected.id}
              description={selected.id}
              actions={<>
                <label className="settings-switch">
                  <input type="checkbox" checked={selected.exposed !== false} onChange={(event) => updateSelected({ exposed: event.target.checked })} />
                  <span />
                  <strong>{selected.exposed !== false ? "已暴露" : "未暴露"}</strong>
                </label>
                {!BUILTIN_IDS.has(selected.id) && (
                  <Button className="settings-danger" onClick={() => void removeSelected()}>
                    <Trash2 size={14} />删除档案
                  </Button>
                )}
              </>}
            />
            <div className="settings-form-grid">
              <label className="settings-field">
                <span>显示名称</span>
                <input value={selected.name} onChange={(event) => updateSelected({ name: event.target.value })} />
                <small>用于子 Agent 选择和运行状态展示</small>
              </label>
              <AgentRuntimeFields
                config={config}
                providerId={selected.provider_id || ""}
                model={selected.model || ""}
                thinkingLevel={selected.thinking_level || "auto"}
                inheritedProviderId={subagent.provider_id || config.active_provider}
                inheritProviderLabel="沿用主 Agent"
                providerHelp="留空时继承子 Agent 默认项或主 Agent 的供应商"
                defaultModelLabel={subagent.model ? "沿用子 Agent 默认模型" : "供应商默认"}
                thinkingHelp="覆盖主 Agent 的推理强度"
                onChange={updateSelected}
              />
              <label className="settings-field full">
                <span>用途描述</span>
                <input value={selected.description ?? ""} onChange={(event) => updateSelected({ description: event.target.value })} />
                <small>主 Agent 根据这段描述判断是否委派任务</small>
              </label>
              <label className="settings-field full subagent-prompt-field">
                <span>系统提示词</span>
                <TextArea value={selected.system_prompt ?? ""} onChange={(event) => updateSelected({ system_prompt: event.target.value })} placeholder="留空使用内置提示词" />
                <small>约束该子 Agent 的职责、边界和输出格式</small>
              </label>
            </div>
          </section>
        ) : (
          <div className="settings-empty">没有可编辑的子 Agent</div>
        )}
      </div>
    </div>
  );
}
