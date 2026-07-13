import type { AppConfig, RunMode } from "../../../api/contracts";
import { Select } from "../../../shared/ui/select/select";
import { SettingsGroup } from "../editor-layout";

const PERMISSION_OPTIONS = [
  { value: "audited", label: "审计", description: "写入工具逐次询问，并限制在工作区沙盒内。" },
  { value: "plan", label: "规划", description: "仅允许只读工具，禁止修改文件和执行写操作。" },
  { value: "yolo", label: "YOLO", description: "不询问工具权限，直接执行允许的工具。" }
] satisfies Array<{ value: RunMode; label: string; description: string }>;

type PermissionDefaultSettingsProps = {
  config: AppConfig;
  onConfigChange: (config: AppConfig) => void;
};

/**
 * 渲染 CLI 与 TUI 共用的默认权限模式设置。
 *
 * @param props 应用配置和更新回调
 * @returns 默认权限模式设置分组
 */
export function PermissionDefaultSettings({ config, onConfigChange }: PermissionDefaultSettingsProps) {
  const value = config.permission?.default_mode ?? "yolo";

  /**
   * 更新共享配置文件中的终端默认权限模式。
   *
   * @param defaultMode 新默认权限模式
   * @returns 无返回值
   */
  const updateDefaultMode = (defaultMode: RunMode) => {
    onConfigChange({ ...config, permission: { default_mode: defaultMode } });
  };

  return (
    <SettingsGroup title="默认权限" description="CLI 与 TUI 未传入模式参数时使用此配置。">
      <div className="settings-form-grid">
        <div className="settings-field">
          <span>终端默认模式</span>
          <Select value={value} options={PERMISSION_OPTIONS} onChange={updateDefaultMode} ariaLabel="终端默认权限模式" menuPreferredWidth={330} />
          <small>终端配置界面和 Web 设置页写入同一份 Miyu 配置。</small>
        </div>
      </div>
    </SettingsGroup>
  );
}
