import { Braces } from "lucide-react";

type AdvancedSettingsSectionProps = {
  value: string;
  onChange: (value: string) => void;
};

/**
 * 渲染完整 AppConfig JSON 编辑器。
 *
 * @param props JSON 文本和更新回调
 * @returns 高级设置区域
 */
export function AdvancedSettingsSection({ value, onChange }: AdvancedSettingsSectionProps) {
  return (
    <section className="settings-panel advanced-settings-panel">
      <header className="settings-panel-header">
        <div><span className="settings-kicker">完整配置</span><h2>高级 JSON</h2><p>保存时服务端会重新反序列化、合并敏感字段并执行完整校验。</p></div>
        <span className="advanced-settings-icon"><Braces size={17} /></span>
      </header>
      <div className="advanced-settings-note">结构化设置尚未覆盖的工具、显示、提示词和插件选项可在此修改。</div>
      <textarea value={value} onChange={(event) => onChange(event.target.value)} spellCheck={false} aria-label="完整 AppConfig JSON" />
    </section>
  );
}
