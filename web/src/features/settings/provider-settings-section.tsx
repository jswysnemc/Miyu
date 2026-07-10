import { Check, Cpu, KeyRound, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../../api/client";
import type { AppConfig, ProviderConfig } from "../../api/contracts";
import { ModelMetadataEditor } from "./model-metadata-editor";

type ProviderSettingsSectionProps = {
  config: AppConfig;
  onConfigChange: (config: AppConfig) => void;
  onProviderChange: (index: number, patch: Partial<ProviderConfig>) => void;
};

/**
 * 渲染供应商列表和当前供应商编辑表单。
 *
 * @param props 应用配置和更新回调
 * @returns 供应商设置区域
 */
export function ProviderSettingsSection({ config, onConfigChange, onProviderChange }: ProviderSettingsSectionProps) {
  const [selectedId, setSelectedId] = useState(config.active_provider || config.providers[0]?.id || "");
  const [fetching, setFetching] = useState(false);
  const [fetchError, setFetchError] = useState("");
  const selectedIndex = Math.max(0, config.providers.findIndex((provider) => provider.id === selectedId));
  const provider = config.providers[selectedIndex];

  useEffect(() => {
    if (!config.providers.some((item) => item.id === selectedId)) {
      setSelectedId(config.active_provider || config.providers[0]?.id || "");
    }
  }, [config.active_provider, config.providers, selectedId]);

  /** 新增一项 OpenAI 兼容供应商草稿。 */
  const addProvider = () => {
    let suffix = 1;
    let id = "provider";
    while (config.providers.some((item) => item.id === id)) {
      suffix += 1;
      id = `provider-${suffix}`;
    }
    const next: ProviderConfig = {
      id,
      display_name: "新供应商",
      base_url: "https://api.example.com/v1",
      protocol: "auto",
      api_key: "",
      models: [],
      default_model: "",
      thinking_level: "auto",
      thinking_format: "auto"
    };
    onConfigChange({ ...config, providers: [...config.providers, next] });
    setSelectedId(id);
    setFetchError("");
  };

  /** 获取当前供应商模型并同步配置草稿。 */
  const fetchModels = async () => {
    if (!provider) return;
    setFetching(true);
    setFetchError("");
    try {
      const response = await api.providers.models(provider);
      onProviderChange(selectedIndex, {
        models: response.models,
        default_model: provider.default_model || response.models[0] || ""
      });
    } catch (error) {
      setFetchError(error instanceof Error ? error.message : String(error));
    } finally {
      setFetching(false);
    }
  };

  /** 删除当前供应商并选择剩余首项。 */
  const deleteProvider = () => {
    if (!provider || !window.confirm(`确定删除供应商“${provider.display_name || provider.id}”吗？`)) return;
    const providers = config.providers.filter((_, index) => index !== selectedIndex);
    const activeProvider = config.active_provider === provider.id ? providers[0]?.id ?? "" : config.active_provider;
    onConfigChange({ ...config, providers, active_provider: activeProvider });
    setSelectedId(activeProvider || providers[0]?.id || "");
  };

  if (!provider) {
    return <div className="settings-empty"><button type="button" className="settings-secondary" onClick={addProvider}><Plus size={14} />新增供应商</button></div>;
  }

  return (
    <div className="provider-settings-layout">
      <aside className="provider-list" aria-label="供应商列表">
        <div className="settings-subheading"><span>供应商</span><span className="provider-list-actions"><small>{config.providers.length}</small><button type="button" onClick={addProvider} aria-label="新增供应商"><Plus size={13} /></button></span></div>
        {config.providers.map((item) => (
          <button type="button" className={item.id === selectedId ? "active" : ""} key={item.id} onClick={() => { setSelectedId(item.id); setFetchError(""); }}>
            <span className="provider-list-icon"><Cpu size={14} /></span>
            <span><strong>{item.display_name || item.id}</strong><small>{item.default_model || item.models?.[0] || "未配置模型"}</small></span>
            {item.id === config.active_provider && <Check size={14} className="provider-active-check" />}
          </button>
        ))}
      </aside>
      <section className="settings-panel provider-editor">
        <header className="settings-panel-header">
          <div><span className="settings-kicker">模型供应商</span><h2>{provider.display_name || provider.id}</h2><p>配置接口、凭据和当前供应商可用的模型。</p></div>
          <div className="provider-header-actions">
            <button type="button" className="settings-secondary" onClick={() => void fetchModels()} disabled={fetching || !provider.base_url.trim()}><RefreshCw size={14} className={fetching ? "spin" : ""} />{fetching ? "正在获取" : "获取模型"}</button>
            <button type="button" className={provider.id === config.active_provider ? "settings-secondary active" : "settings-secondary"} onClick={() => onConfigChange({ ...config, active_provider: provider.id })} disabled={provider.id === config.active_provider}><Check size={14} />{provider.id === config.active_provider ? "当前供应商" : "设为当前"}</button>
            <button type="button" className="settings-secondary danger" onClick={deleteProvider}><Trash2 size={14} />删除</button>
          </div>
        </header>
        {fetchError && <div className="provider-fetch-error">{fetchError}</div>}
        <div className="settings-form-grid">
          <label className="settings-field"><span>供应商 ID</span><input value={provider.id} onChange={(event) => { setSelectedId(event.target.value); onProviderChange(selectedIndex, { id: event.target.value }); }} /><small>配置文件中的稳定标识</small></label>
          <label className="settings-field"><span>显示名称</span><input value={provider.display_name} onChange={(event) => onProviderChange(selectedIndex, { display_name: event.target.value })} /><small>用于模型菜单和状态展示</small></label>
          <label className="settings-field full"><span>API 地址</span><input value={provider.base_url} onChange={(event) => onProviderChange(selectedIndex, { base_url: event.target.value })} spellCheck={false} /><small>兼容接口的基础地址，获取模型时由服务端访问</small></label>
          <label className="settings-field"><span>协议</span><select value={provider.protocol ?? "auto"} onChange={(event) => onProviderChange(selectedIndex, { protocol: event.target.value })}><option value="auto">自动检测</option><option value="openai-chat">OpenAI Chat Completions</option><option value="openai-responses">OpenAI Responses</option><option value="anthropic">Anthropic Messages</option></select><small>协议决定请求和思考参数格式</small></label>
          <label className="settings-field"><span>默认模型</span><input value={provider.default_model ?? ""} onChange={(event) => onProviderChange(selectedIndex, { default_model: event.target.value })} /><small>未手动切换时使用</small></label>
          <label className="settings-field full"><span>API Key</span><div className="secret-input"><KeyRound size={14} /><input type="password" value={provider.api_key ?? ""} onChange={(event) => onProviderChange(selectedIndex, { api_key: event.target.value })} autoComplete="off" /></div><small>支持使用 `$env:VARIABLE_NAME` 引用环境变量</small></label>
          <label className="settings-field"><span>请求超时</span><input type="number" min="1" value={provider.timeout_seconds ?? 120} onChange={(event) => onProviderChange(selectedIndex, { timeout_seconds: Number(event.target.value) })} /><small>单位为秒</small></label>
          <label className="settings-field"><span>Temperature</span><input type="number" min="0" max="2" step="0.1" value={provider.temperature ?? 0.7} onChange={(event) => onProviderChange(selectedIndex, { temperature: Number(event.target.value) })} /><small>模型采样温度</small></label>
          <label className="settings-field"><span>思考等级</span><select value={provider.thinking_level ?? "auto"} onChange={(event) => onProviderChange(selectedIndex, { thinking_level: event.target.value })}><option value="auto">自动</option><option value="max">最大</option><option value="xhigh">极高</option><option value="high">高</option><option value="medium">中</option><option value="low">低</option><option value="none">关闭</option></select><small>供应商默认推理强度</small></label>
          <label className="settings-field"><span>思考格式</span><select value={provider.thinking_format ?? "auto"} onChange={(event) => onProviderChange(selectedIndex, { thinking_format: event.target.value })}><option value="auto">自动</option><option value="reasoning_content">reasoning_content</option><option value="reasoning">reasoning</option><option value="thinking">thinking</option></select><small>响应中的思考字段</small></label>
          <label className="settings-field"><span>Anthropic max_tokens</span><input type="number" min="1" value={provider.anthropic_max_tokens ?? 8192} onChange={(event) => onProviderChange(selectedIndex, { anthropic_max_tokens: Number(event.target.value) })} /><small>仅 Anthropic Messages 使用</small></label>
          <label className="settings-field full"><span>自定义 body JSON</span><textarea rows={5} value={provider.extra_body ?? ""} onChange={(event) => onProviderChange(selectedIndex, { extra_body: event.target.value })} spellCheck={false} placeholder={'{"max_tokens": 4096}'} /><small>对象会合并到每次模型请求，显式配置字段优先</small></label>
        </div>
        <ModelMetadataEditor provider={provider} onChange={(patch) => onProviderChange(selectedIndex, patch)} />
      </section>
    </div>
  );
}
