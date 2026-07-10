import { Braces, Cable, KeyRound, MessageSquareText, Palette, Plug, Save, Settings2, SlidersHorizontal } from "lucide-react";
import { useState } from "react";
import { AdvancedSettingsSection } from "./advanced-settings-section";
import { AppearanceSettingsSection } from "./appearance-settings-section";
import { GatewaySettingsSection } from "./gateway-settings-section";
import { ProviderSettingsSection } from "./provider-settings-section";
import { PluginSettingsSection } from "./plugin-settings-section";
import { PromptSettingsSection } from "./prompt-settings-section";
import { RuntimeSettingsSection } from "./runtime-settings-section";
import type { SettingsSectionId } from "./settings-types";
import { useSettingsConfig } from "./use-settings-config";
import { useTheme } from "../theme/theme";
import "./settings-page.css";
import "./settings-extended.css";
import "./settings-refined.css";

const sections = [
  { id: "providers", label: "供应商与模型", description: "接口、凭据和模型列表", icon: KeyRound },
  { id: "plugins", label: "插件配置", description: "搜索、视觉、知识库和记忆", icon: Plug },
  { id: "prompts", label: "提示词", description: "AI 人设和用户身份", icon: MessageSquareText },
  { id: "runtime", label: "运行参数", description: "工具、技能、显示和上下文", icon: SlidersHorizontal },
  { id: "appearance", label: "主题与配色", description: "界面主题和颜色方案", icon: Palette },
  { id: "gateways", label: "消息网关", description: "QQ、微信和运行状态", icon: Cable },
  { id: "advanced", label: "高级配置", description: "完整 AppConfig JSON", icon: Braces }
] satisfies Array<{ id: SettingsSectionId; label: string; description: string; icon: typeof KeyRound }>;

/** 渲染分类式专业配置管理界面。 */
export function SettingsPage() {
  const settings = useSettingsConfig();
  const theme = useTheme();
  const [section, setSection] = useState<SettingsSectionId>("providers");
  const activeSection = sections.find((item) => item.id === section)!;

  return (
    <div className="settings-page">
      <header className="settings-header">
        <div className="settings-title-icon"><Settings2 size={21} /></div>
        <div><span className="settings-kicker">应用配置</span><h1>设置</h1><p>管理模型、插件、提示词、工具、网关和界面偏好。</p></div>
        <div className="settings-save-area">
          {settings.dirty && <span className="settings-dirty">存在未保存修改</span>}
          <button type="button" className="settings-save" onClick={() => void settings.saveConfig()} disabled={!settings.config || !settings.dirty || settings.saving}><Save size={15} />{settings.saving ? "正在保存" : "保存修改"}</button>
        </div>
      </header>
      <div className="settings-workspace">
        <nav className="settings-navigation" aria-label="设置分类">
          <div className="settings-navigation-label">设置分类</div>
          {sections.map(({ id, label, description, icon: Icon }) => (
            <button type="button" key={id} className={id === section ? "active" : ""} onClick={() => setSection(id)}>
              <span><Icon size={16} /></span>
              <span><strong>{label}</strong><small>{description}</small></span>
            </button>
          ))}
        </nav>
        <main className="settings-main">
          <div className="settings-section-title"><h2>{activeSection.label}</h2><p>{activeSection.description}</p></div>
          {settings.loading && <div className="settings-state">正在读取配置</div>}
          {settings.config && section === "providers" && <ProviderSettingsSection config={settings.config} onConfigChange={settings.updateConfig} onProviderChange={settings.updateProvider} />}
          {settings.config && section === "plugins" && <PluginSettingsSection config={settings.config} onConfigChange={settings.updateConfig} />}
          {settings.config && section === "prompts" && <PromptSettingsSection config={settings.config} onConfigChange={settings.updateConfig} />}
          {settings.config && section === "runtime" && <RuntimeSettingsSection config={settings.config} onConfigChange={settings.updateConfig} />}
          {section === "appearance" && <AppearanceSettingsSection theme={theme.theme} onThemeChange={theme.setTheme} />}
          {settings.config && section === "gateways" && <GatewaySettingsSection config={settings.config} dirty={settings.dirty} onGatewayChange={settings.updateGateway} onSave={settings.saveConfig} />}
          {settings.config && section === "advanced" && <AdvancedSettingsSection value={settings.raw} onChange={settings.updateRaw} />}
          {settings.error && <div className="settings-error">{settings.error.message}</div>}
          {settings.saved && !settings.dirty && <div className="settings-success">配置已经保存并通过校验</div>}
        </main>
      </div>
    </div>
  );
}
