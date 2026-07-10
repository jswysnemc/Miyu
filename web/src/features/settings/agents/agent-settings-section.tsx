import { useEffect, useMemo, useRef, useState } from "react";
import type { AppConfig } from "../../../api/contracts";
import { useConfirm } from "../../../shared/ui/dialog/dialog-provider";
import { fetchAgentOptions } from "./agents-api";
import type { AgentOptions, AgentProfile, AgentToolOption } from "./agents-types";
import { SkillAssignmentBoard } from "./skill-assignment-board";
import "./agents.css";

type AgentSettingsSectionProps = {
  /** 当前应用配置 */
  config: AppConfig;
  /** 配置更新回调 */
  onConfigChange: (config: AppConfig) => void;
};

/** 从配置中读取 agents 数组，缺失时返回空数组。 */
function readAgents(config: AppConfig): AgentProfile[] {
  const value = (config as { agents?: AgentProfile[] }).agents;
  return Array.isArray(value) ? value : [];
}

/**
 * 渲染 Agent 配置区域：左侧档案列表，右侧编辑器。
 *
 * @param props 应用配置和更新回调
 * @returns Agent 设置区域
 */
export function AgentSettingsSection({ config, onConfigChange }: AgentSettingsSectionProps) {
  const confirm = useConfirm();
  const agents = readAgents(config);
  const [selectedId, setSelectedId] = useState(agents[0]?.id ?? "");
  const [options, setOptions] = useState<AgentOptions>({ tools: [], skills: [] });
  const [optionsError, setOptionsError] = useState("");
  const selected = agents.find((agent) => agent.id === selectedId) ?? agents[0] ?? null;

  useEffect(() => {
    let cancelled = false;
    fetchAgentOptions()
      .then((data) => {
        if (!cancelled) setOptions(data);
      })
      .catch((error: Error) => {
        if (!cancelled) setOptionsError(error.message);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  /** 写回整份 agents 数组。 */
  const setAgents = (next: AgentProfile[]) => {
    onConfigChange({ ...config, agents: next } as AppConfig);
  };

  /** 更新当前选中 agent 的部分字段。 */
  const updateSelected = (patch: Partial<AgentProfile>) => {
    if (!selected) return;
    setAgents(agents.map((agent) => (agent.id === selected.id ? { ...agent, ...patch } : agent)));
  };

  /** 新增一个空白 agent 档案并选中。 */
  const addAgent = () => {
    let suffix = agents.length + 1;
    let id = `agent-${suffix}`;
    while (agents.some((agent) => agent.id === id)) {
      suffix += 1;
      id = `agent-${suffix}`;
    }
    const next: AgentProfile = {
      id,
      name: `新 Agent ${suffix}`,
      system_prompt: "",
      enabled_tools: [],
      skills_full: [],
      skills_named: []
    };
    setAgents([...agents, next]);
    setSelectedId(id);
  };

  /** 删除当前选中 agent，需要确认。 */
  const removeSelected = async () => {
    if (!selected) return;
    const confirmed = await confirm({
      title: "删除 Agent",
      description: `将删除“${selected.name || selected.id}”的全部配置。`,
      confirmLabel: "删除 Agent",
      danger: true
    });
    if (!confirmed) return;
    const next = agents.filter((agent) => agent.id !== selected.id);
    setAgents(next);
    setSelectedId(next[0]?.id ?? "");
  };

  return (
    <div className="agent-settings">
      <aside className="agent-list">
        <div className="agent-list-head">
          <span>Agent 档案</span>
          <button type="button" className="agent-list-add" onClick={addAgent}>新增</button>
        </div>
        {agents.length === 0 && <p className="agent-list-empty">尚未创建任何 Agent。</p>}
        {agents.map((agent) => (
          <button
            key={agent.id}
            type="button"
            className={`agent-list-item${selected?.id === agent.id ? " active" : ""}`}
            onClick={() => setSelectedId(agent.id)}
          >
            <span className="agent-list-item-name">{agent.name || agent.id}</span>
            <span className="agent-list-item-meta">
              {agent.enabled_tools.length} 工具 · {agent.skills_full.length + agent.skills_named.length} skills
            </span>
          </button>
        ))}
      </aside>
      {selected ? (
        <div className="agent-editor">
          <div className="agent-editor-row">
            <label className="agent-field">
              <span className="agent-field-label">名称</span>
              <input
                type="text"
                value={selected.name}
                onChange={(event) => updateSelected({ name: event.target.value })}
              />
            </label>
            <button type="button" className="agent-delete" onClick={removeSelected}>删除</button>
          </div>
          <label className="agent-field">
            <span className="agent-field-label">系统提示词</span>
            <textarea
              className="agent-prompt-input"
              value={selected.system_prompt}
              onChange={(event) => updateSelected({ system_prompt: event.target.value })}
              placeholder="为该 Agent 编写系统提示词"
            />
          </label>
          <section className="agent-block">
            <h4 className="agent-block-title">启用工具</h4>
            {optionsError && <p className="agent-options-error">工具列表加载失败：{optionsError}</p>}
            <ToolGroupList
              tools={options.tools}
              enabled={selected.enabled_tools}
              onChange={(enabledTools) => updateSelected({ enabled_tools: enabledTools })}
            />
          </section>
          <section className="agent-block">
            <h4 className="agent-block-title">Skills 暴露</h4>
            <SkillAssignmentBoard
              skills={options.skills}
              fullNames={selected.skills_full}
              namedNames={selected.skills_named}
              onChange={(fullNames, namedNames) =>
                updateSelected({ skills_full: fullNames, skills_named: namedNames })
              }
            />
          </section>
        </div>
      ) : (
        <div className="agent-editor agent-editor-empty">选择或新增一个 Agent 开始配置。</div>
      )}
    </div>
  );
}

type ToolGroupListProps = {
  /** 全部内置工具选项 */
  tools: AgentToolOption[];
  /** 当前启用的工具名集合 */
  enabled: string[];
  /** 启用集合变化回调 */
  onChange: (enabled: string[]) => void;
};

/**
 * 按分组渲染工具复选框列表，组标题复选框可全选或取消本组。
 *
 * @param props 工具选项、启用集合和变化回调
 * @returns 工具分组列表
 */
function ToolGroupList({ tools, enabled, onChange }: ToolGroupListProps) {
  /** 按分组名聚合工具。 */
  const groups = useMemo(() => {
    const map = new Map<string, AgentToolOption[]>();
    for (const tool of tools) {
      const list = map.get(tool.group) ?? [];
      list.push(tool);
      map.set(tool.group, list);
    }
    return [...map.entries()].sort(([left], [right]) => left.localeCompare(right));
  }, [tools]);

  /** 切换单个工具的启用状态。 */
  const toggleTool = (name: string, checked: boolean) => {
    const next = enabled.filter((item) => item !== name);
    if (checked) next.push(name);
    onChange(next);
  };

  /** 全选或取消一个分组下的全部工具。 */
  const toggleGroup = (names: string[], checked: boolean) => {
    const next = enabled.filter((item) => !names.includes(item));
    if (checked) next.push(...names);
    onChange(next);
  };

  if (tools.length === 0) {
    return <p className="agent-tools-empty">暂无可用工具。</p>;
  }
  return (
    <div className="agent-tool-groups">
      {groups.map(([group, items]) => {
        const names = items.map((item) => item.name);
        const checkedCount = names.filter((name) => enabled.includes(name)).length;
        return (
          <fieldset key={group} className="agent-tool-group">
            <legend className="agent-tool-group-head">
              <GroupCheckbox
                allChecked={checkedCount === names.length}
                someChecked={checkedCount > 0 && checkedCount < names.length}
                onToggle={(checked) => toggleGroup(names, checked)}
              />
              <span className="agent-tool-group-name">{group}</span>
              <span className="agent-tool-group-count">{checkedCount}/{names.length}</span>
            </legend>
            <div className="agent-tool-items">
              {items.map((tool) => (
                <label key={tool.name} className="agent-tool-item">
                  <input
                    type="checkbox"
                    checked={enabled.includes(tool.name)}
                    onChange={(event) => toggleTool(tool.name, event.target.checked)}
                  />
                  <span>{tool.name}</span>
                </label>
              ))}
            </div>
          </fieldset>
        );
      })}
    </div>
  );
}

type GroupCheckboxProps = {
  /** 是否全部选中 */
  allChecked: boolean;
  /** 是否部分选中 */
  someChecked: boolean;
  /** 切换回调，参数为切换后是否全选 */
  onToggle: (checked: boolean) => void;
};

/**
 * 支持半选状态的分组复选框。
 *
 * @param props 选中状态和切换回调
 * @returns 分组复选框
 */
function GroupCheckbox({ allChecked, someChecked, onToggle }: GroupCheckboxProps) {
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (ref.current) ref.current.indeterminate = someChecked;
  }, [someChecked]);
  return (
    <input
      ref={ref}
      type="checkbox"
      checked={allChecked}
      onChange={(event) => onToggle(event.target.checked)}
    />
  );
}
