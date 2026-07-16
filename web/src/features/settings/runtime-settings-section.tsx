import type { AppConfig } from "../../api/contracts";
import { SettingsGroup } from "./editor-layout";
import { StructuredConfigFields } from "./structured-config-fields";
import { PermissionDefaultSettings } from "./runtime/permission-default-settings";
import { TerminalSettingsFields } from "./terminal-settings-fields";

type RuntimeSettingsSectionProps = {
  config: AppConfig;
  onConfigChange: (config: AppConfig) => void;
};

/**
 * 渲染工具、技能、显示和上下文运行参数。
 *
 * @param props 应用配置和更新回调
 * @returns 运行参数设置区域
 */
export function RuntimeSettingsSection({ config, onConfigChange }: RuntimeSettingsSectionProps) {
  const groups = [
    ["tools", "工具执行", "控制工具轮次、Shell 和后台命令。"],
    ["skills", "技能系统", "控制技能加载和命令执行权限。"],
    ["display", "输出显示", "控制思考、工具调用和等待状态。"],
    ["context", "上下文管理", "配置模型窗口和自动压缩阈值。"]
  ] as const;
  return (
    <div className="runtime-groups">
      <PermissionDefaultSettings config={config} onConfigChange={onConfigChange} />
      <SettingsGroup title="网页终端" description="配置网页终端启动的 Shell，新建终端时生效。">
        <TerminalSettingsFields config={config} onConfigChange={onConfigChange} />
      </SettingsGroup>
      {groups.map(([key, title, description]) => (
        <SettingsGroup title={title} description={description} key={key}>
          <StructuredConfigFields
            value={(config[key] as Record<string, unknown> | undefined) ?? {}}
            onChange={(next) => onConfigChange({ ...config, [key]: next })}
          />
        </SettingsGroup>
      ))}
    </div>
  );
}
