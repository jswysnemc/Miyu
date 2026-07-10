import { Plus, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import type { ModelMetadata, ProviderConfig } from "../../api/contracts";

const MODEL_TAGS = ["tool", "thinking", "vision", "web_search", "fast", "low_cost"];

type ModelMetadataEditorProps = {
  provider: ProviderConfig;
  onChange: (patch: Partial<ProviderConfig>) => void;
};

/**
 * 渲染模型列表、默认模型和单模型能力元数据。
 *
 * @param props 供应商配置和更新回调
 * @returns 模型目录编辑器
 */
export function ModelMetadataEditor({ provider, onChange }: ModelMetadataEditorProps) {
  const models = provider.models ?? [];
  const [selected, setSelected] = useState(provider.default_model || models[0] || "");
  const [draft, setDraft] = useState("");

  useEffect(() => {
    if (!models.includes(selected)) setSelected(provider.default_model || models[0] || "");
  }, [models.join("\u0000"), provider.default_model, selected]);

  const metadata = provider.model_metadata?.[selected] ?? {};

  /** 新增模型标识并选中。 */
  const addModel = () => {
    const model = draft.trim();
    if (!model || models.includes(model)) return;
    onChange({ models: [...models, model], default_model: provider.default_model || model });
    setSelected(model);
    setDraft("");
  };

  /** 删除当前模型及其元数据。 */
  const removeModel = (model: string) => {
    const nextModels = models.filter((item) => item !== model);
    const nextMetadata = { ...(provider.model_metadata ?? {}) };
    delete nextMetadata[model];
    onChange({
      models: nextModels,
      default_model: provider.default_model === model ? nextModels[0] ?? "" : provider.default_model,
      model_metadata: nextMetadata
    });
  };

  /** 更新当前模型元数据。 */
  const updateMetadata = (patch: Partial<ModelMetadata>) => {
    if (!selected) return;
    onChange({
      model_metadata: {
        ...(provider.model_metadata ?? {}),
        [selected]: { ...metadata, ...patch }
      }
    });
  };

  /** 切换当前模型标签。 */
  const toggleTag = (tag: string) => {
    const tags = metadata.tags ?? [];
    updateMetadata({ tags: tags.includes(tag) ? tags.filter((item) => item !== tag) : [...tags, tag] });
  };

  return (
    <section className="model-catalog">
      <header><div><span>模型目录</span><small>{models.length} 个模型</small></div><div className="model-add"><input value={draft} onChange={(event) => setDraft(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); addModel(); } }} placeholder="新增模型 ID" /><button type="button" onClick={addModel} aria-label="新增模型"><Plus size={14} /></button></div></header>
      <div className="model-catalog-body">
        <div className="model-chip-list">
          {models.map((model) => (
            <div className={model === selected ? "model-chip active" : "model-chip"} key={model}>
              <button type="button" onClick={() => setSelected(model)}>{model}</button>
              <button type="button" onClick={() => removeModel(model)} aria-label={`删除模型 ${model}`}><Trash2 size={12} /></button>
            </div>
          ))}
        </div>
        {selected && (
          <div className="model-metadata-form">
            <div className="model-metadata-head"><div><strong>{selected}</strong><small>单模型能力与上下文</small></div><button type="button" className={provider.default_model === selected ? "settings-secondary active" : "settings-secondary"} onClick={() => onChange({ default_model: selected })}>{provider.default_model === selected ? "默认模型" : "设为默认"}</button></div>
            <div className="settings-form-grid">
              <label className="settings-field"><span>上下文 token 数</span><input type="number" min="1" value={metadata.context_chars ?? ""} onChange={(event) => updateMetadata({ context_chars: event.target.value ? Number(event.target.value) : undefined })} placeholder="例如 128000" /><small>支持模型级上下文窗口</small></label>
              <label className="settings-field"><span>工具调用</span><select value={metadata.tools_enabled === false ? "disabled" : "enabled"} onChange={(event) => updateMetadata({ tools_enabled: event.target.value === "enabled" ? undefined : false })}><option value="enabled">允许</option><option value="disabled">禁用</option></select><small>覆盖供应商默认能力</small></label>
            </div>
            <div className="model-tag-field"><span>模型标签</span><div>{MODEL_TAGS.map((tag) => <button type="button" className={(metadata.tags ?? []).includes(tag) ? "active" : ""} key={tag} onClick={() => toggleTag(tag)}>{tag}</button>)}</div></div>
          </div>
        )}
      </div>
    </section>
  );
}
