import { createElement } from "react";
import type { AppConfig } from "../../../api/contracts";
import { ModelIcon } from "../../../shared/ui/model-icon";
import { Select } from "../../../shared/ui/select/select";

type AgentRuntimePatch = {
  provider_id?: string;
  model?: string;
  thinking_level?: string;
};

type AgentRuntimeFieldsProps = {
  /** 应用配置 */
  config: AppConfig;
  /** 当前独立供应商标识 */
  providerId: string;
  /** 当前独立模型 */
  model: string;
  /** 当前思考等级 */
  thinkingLevel: string;
  /** 独立供应商留空时继承的供应商标识 */
  inheritedProviderId?: string;
  /** 供应商继承选项文案 */
  inheritProviderLabel: string;
  /** 供应商字段说明 */
  providerHelp: string;
  /** 空模型选项文案 */
  defaultModelLabel: string;
  /** 思考等级字段说明 */
  thinkingHelp: string;
  /** 运行参数变化回调 */
  onChange: (patch: AgentRuntimePatch) => void;
};

const THINKING_OPTIONS = ["auto", "none", "low", "medium", "high", "xhigh", "max"]
  .map((value) => ({ value, label: value }));

/**
 * 构造当前运行覆盖可选择的模型名称，并保留模型列表外的历史配置值。
 *
 * @param config 应用配置
 * @param providerId 当前独立供应商标识
 * @param inheritedProviderId 独立供应商留空时继承的供应商标识
 * @param currentModel 当前模型覆盖值
 * @returns 去重后的模型名称数组
 */
export function buildAgentModelNames(
  config: AppConfig,
  providerId: string,
  inheritedProviderId: string | undefined,
  currentModel: string
): string[] {
  const effectiveProviderId = providerId || inheritedProviderId || config.active_provider;
  const provider = config.providers.find((item) => item.id === effectiveProviderId);
  return [...new Set([currentModel, ...(provider?.models ?? [])].filter(Boolean))];
}

/**
 * 渲染主 Agent 与子 Agent 共用的供应商、模型和思考等级字段。
 *
 * @param props 运行覆盖值、继承规则、字段文案和更新回调
 * @returns 三个运行参数表单字段
 */
export function AgentRuntimeFields({
  config,
  providerId,
  model,
  thinkingLevel,
  inheritedProviderId,
  inheritProviderLabel,
  providerHelp,
  defaultModelLabel,
  thinkingHelp,
  onChange
}: AgentRuntimeFieldsProps) {
  const modelNames = buildAgentModelNames(config, providerId, inheritedProviderId, model);
  const effectiveProviderId = providerId || inheritedProviderId || config.active_provider;
  const provider = config.providers.find((item) => item.id === effectiveProviderId);

  return <>
    <div className="settings-field">
      <span>供应商</span>
      <Select
        value={providerId}
        options={[{ value: "", label: inheritProviderLabel }, ...config.providers.map((item) => ({ value: item.id, label: item.display_name || item.id }))]}
        onChange={(value) => onChange({ provider_id: value, model: "" })}
        ariaLabel="Agent 供应商"
      />
      <small>{providerHelp}</small>
    </div>
    <div className="settings-field">
      <span>模型</span>
      <Select
        value={model}
        options={[
          { value: "", label: defaultModelLabel },
          ...modelNames.map((name) => ({ value: name, label: name, icon: createElement(ModelIcon, { model: name, size: 14 }) }))
        ]}
        onChange={(value) => onChange({ model: value })}
        disabled={!provider && modelNames.length === 0}
        ariaLabel="Agent 模型"
      />
      <small>{provider ? `使用${provider.display_name || provider.id}的模型列表` : "当前没有可用供应商"}</small>
    </div>
    <div className="settings-field">
      <span>思考等级</span>
      <Select value={thinkingLevel || "auto"} options={THINKING_OPTIONS} onChange={(value) => onChange({ thinking_level: value })} ariaLabel="Agent 思考等级" />
      <small>{thinkingHelp}</small>
    </div>
  </>;
}
