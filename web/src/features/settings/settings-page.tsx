import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Braces, KeyRound, Save, ServerCog } from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../../api/client";
import "./settings-page.css";

type Provider = {
  id?: string;
  display_name?: string;
  base_url?: string;
  api_key?: string;
  default_model?: string;
  models?: string[];
};

/** 渲染供应商表单和完整 JSON 配置编辑器。 */
export function SettingsPage() {
  const queryClient = useQueryClient();
  const response = useQuery({ queryKey: ["config"], queryFn: api.config.load });
  const [config, setConfig] = useState<Record<string, unknown> | null>(null);
  const [raw, setRaw] = useState("");
  const [tab, setTab] = useState<"providers" | "advanced">("providers");
  useEffect(() => {
    if (response.data) {
      setConfig(response.data.config);
      setRaw(JSON.stringify(response.data.config, null, 2));
    }
  }, [response.data]);
  const save = useMutation({
    mutationFn: async () => {
      const value = tab === "advanced" ? JSON.parse(raw) as Record<string, unknown> : config!;
      return api.config.save(value);
    },
    onSuccess: async (saved) => {
      setConfig(saved.config);
      setRaw(JSON.stringify(saved.config, null, 2));
      await queryClient.invalidateQueries({ queryKey: ["config"] });
    }
  });
  const providers = Array.isArray(config?.providers) ? config.providers as Provider[] : [];
  const activeProvider = String(config?.active_provider ?? "");

  /** 同步结构化表单与高级 JSON 文本。 */
  const updateConfig = (updated: Record<string, unknown>) => {
    setConfig(updated);
    setRaw(JSON.stringify(updated, null, 2));
  };

  /** 更新指定供应商配置。 */
  const updateProvider = (index: number, patch: Partial<Provider>) => {
    if (!config) return;
    const next = providers.map((provider, providerIndex) => providerIndex === index ? { ...provider, ...patch } : provider);
    const updated = { ...config, providers: next };
    updateConfig(updated);
  };

  return (
    <div className="management-page">
      <header className="management-hero">
        <div className="hero-icon"><ServerCog size={24} /></div>
        <div><span className="eyebrow">Miyu configuration</span><h1>配置管理</h1><p>修改供应商、模型、工具、显示和网关配置。敏感字段不会由服务端返回明文。</p></div>
        <button type="button" className="button primary save-config" onClick={() => save.mutate()} disabled={!config || save.isPending}><Save size={15} /> 保存配置</button>
      </header>
      <div className="settings-tabs">
        <button type="button" className={tab === "providers" ? "active" : ""} onClick={() => setTab("providers")}><KeyRound size={15} />供应商</button>
        <button type="button" className={tab === "advanced" ? "active" : ""} onClick={() => setTab("advanced")}><Braces size={15} />高级 JSON</button>
      </div>
      <div className="settings-content">
        {response.isLoading && <div className="settings-state">正在读取配置</div>}
        {config && tab === "providers" && (
          <section className="provider-section">
            <label className="field active-provider-field"><span>当前供应商</span><select value={activeProvider} onChange={(event) => updateConfig({ ...config, active_provider: event.target.value })}>{providers.map((provider) => <option key={provider.id} value={provider.id}>{provider.display_name || provider.id}</option>)}</select></label>
            <div className="provider-grid">
              {providers.map((provider, index) => (
                <article className={provider.id === activeProvider ? "provider-card active" : "provider-card"} key={`${provider.id}-${index}`}>
                  <div className="provider-card-head"><span>{String(index + 1).padStart(2, "0")}</span><strong>{provider.display_name || provider.id || "未命名供应商"}</strong></div>
                  <div className="field-grid">
                    <label className="field"><span>ID</span><input value={provider.id ?? ""} onChange={(event) => updateProvider(index, { id: event.target.value })} /></label>
                    <label className="field"><span>名称</span><input value={provider.display_name ?? ""} onChange={(event) => updateProvider(index, { display_name: event.target.value })} /></label>
                    <label className="field full"><span>API 地址</span><input value={provider.base_url ?? ""} onChange={(event) => updateProvider(index, { base_url: event.target.value })} /></label>
                    <label className="field"><span>默认模型</span><input value={provider.default_model ?? ""} onChange={(event) => updateProvider(index, { default_model: event.target.value })} /></label>
                    <label className="field"><span>API Key</span><input type="password" value={provider.api_key ?? ""} onChange={(event) => updateProvider(index, { api_key: event.target.value })} /></label>
                    <label className="field full"><span>模型列表，每行一个</span><textarea rows={4} value={(provider.models ?? []).join("\n")} onChange={(event) => updateProvider(index, { models: event.target.value.split("\n").map((value) => value.trim()).filter(Boolean) })} /></label>
                  </div>
                </article>
              ))}
            </div>
          </section>
        )}
        {config && tab === "advanced" && <section className="advanced-config"><div className="advanced-note">高级编辑器覆盖完整 AppConfig。服务端会重新执行反序列化和配置校验。</div><textarea value={raw} onChange={(event) => setRaw(event.target.value)} spellCheck={false} /></section>}
        {(response.error || save.error) && <div className="settings-error">{(response.error ?? save.error)?.message}</div>}
        {save.isSuccess && <div className="settings-success">配置已经保存</div>}
      </div>
    </div>
  );
}
