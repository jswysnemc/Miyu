import { Check, Plug } from "lucide-react";
import { useEffect, useState } from "react";
import type { AppConfig } from "../../api/contracts";
import { StructuredConfigFields } from "./structured-config-fields";

type PluginSettingsSectionProps = {
  config: AppConfig;
  onConfigChange: (config: AppConfig) => void;
};

/**
 * 渲染全部插件的分类列表和结构化配置。
 *
 * @param props 应用配置和更新回调
 * @returns 插件配置区域
 */
export function PluginSettingsSection({ config, onConfigChange }: PluginSettingsSectionProps) {
  const plugins = config.plugins ?? {};
  const names = Object.keys(plugins);
  const [selected, setSelected] = useState(names[0] ?? "");

  useEffect(() => {
    if (!names.includes(selected)) setSelected(names[0] ?? "");
  }, [names.join("\u0000"), selected]);

  const plugin = plugins[selected] ?? {};
  return (
    <div className="provider-settings-layout plugin-settings-layout">
      <aside className="provider-list plugin-list" aria-label="插件列表">
        <div className="settings-subheading"><span>插件</span><small>{names.length}</small></div>
        {names.map((name) => {
          const enabled = plugins[name]?.enabled !== false;
          return (
            <button type="button" className={name === selected ? "active" : ""} key={name} onClick={() => setSelected(name)}>
              <span className="provider-list-icon"><Plug size={14} /></span>
              <span><strong>{pluginLabel(name)}</strong><small>{name}</small></span>
              {enabled && <Check size={13} className="provider-active-check" />}
            </button>
          );
        })}
      </aside>
      <section className="settings-panel provider-editor">
        <header className="settings-panel-header"><div><span className="settings-kicker">插件能力</span><h2>{pluginLabel(selected)}</h2><p>配置开关、服务地址、凭据和插件运行参数。</p></div></header>
        <StructuredConfigFields
          value={plugin}
          onChange={(next) => onConfigChange({ ...config, plugins: { ...plugins, [selected]: next } })}
        />
      </section>
    </div>
  );
}

/**
 * 返回插件中文名称。
 *
 * @param name 插件配置标识
 * @returns 插件显示名称
 */
function pluginLabel(name: string): string {
  const labels: Record<string, string> = {
    weather: "天气查询",
    web: "网页搜索",
    web_images: "网页图片",
    deep_research: "深度研究",
    deep_diagnose: "深度诊断",
    vision: "视觉理解",
    exchange_rate: "汇率查询",
    xuanxue: "六十四卦",
    image_generation: "图片生成",
    print_image: "图片输出",
    memes: "表情图库",
    knowledge_base: "知识库",
    archlinux: "Arch Linux",
    man: "在线手册",
    moegirl: "萌娘百科",
    hash_codec: "哈希编码",
    calculator: "计算器",
    package_advisor: "软件包建议",
    linux_game_compatibility: "Linux 游戏兼容性",
    diagnostics: "运行诊断",
    memory: "长期记忆"
  };
  return labels[name] ?? name.replaceAll("_", " ");
}
