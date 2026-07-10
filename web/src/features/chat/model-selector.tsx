import { Check, ChevronDown, Cpu, Search } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { RunModelSelection } from "../../api/contracts";
import type { ChatModelChoice } from "./chat-model-options";

type ModelSelectorProps = {
  choices: ChatModelChoice[];
  selection: ChatModelChoice | null;
  loading: boolean;
  disabled: boolean;
  onSelect: (selection: RunModelSelection) => void;
};

/**
 * 渲染可搜索并向上展开的供应商模型选择器。
 *
 * @param props 模型选项、当前选择和选择回调
 * @returns 输入区模型选择器
 */
export function ModelSelector({ choices, selection, loading, disabled, onSelect }: ModelSelectorProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ left: 12, bottom: 56, width: 320 });
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const filtered = choices.filter((choice) => (
    !normalizedQuery
    || choice.model.toLocaleLowerCase().includes(normalizedQuery)
    || choice.providerName.toLocaleLowerCase().includes(normalizedQuery)
  ));

  useEffect(() => {
    /** 在模型选择器外点击时关闭菜单。 */
    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!rootRef.current?.contains(target) && !menuRef.current?.contains(target)) setOpen(false);
    };
    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, []);

  useEffect(() => {
    if (!open) return;

    /** 按触发按钮位置计算固定菜单坐标。 */
    const updatePosition = () => {
      const rect = triggerRef.current?.getBoundingClientRect();
      if (!rect) return;
      const viewportPadding = 12;
      const width = Math.min(360, window.innerWidth - viewportPadding * 2);
      const left = Math.max(viewportPadding, Math.min(rect.left, window.innerWidth - width - viewportPadding));
      setPosition({ left, bottom: window.innerHeight - rect.top + 8, width });
    };

    updatePosition();
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [open]);

  /**
   * 选择模型并关闭菜单。
   *
   * @param choice 用户选择的模型
   */
  const handleSelect = (choice: ChatModelChoice) => {
    onSelect({ providerId: choice.providerId, model: choice.model });
    setOpen(false);
    setQuery("");
  };

  return (
    <div className="model-selector" ref={rootRef}>
      <button
        ref={triggerRef}
        type="button"
        className="model-selector-trigger"
        onClick={() => setOpen((value) => !value)}
        disabled={disabled || loading || choices.length === 0}
        aria-haspopup="listbox"
        aria-expanded={open}
      >
        <Cpu size={14} />
        <span>{loading ? "读取模型" : selection?.model ?? "未配置模型"}</span>
        <ChevronDown size={12} className={open ? "model-chevron open" : "model-chevron"} />
      </button>
      {open && createPortal(
        <div className="model-menu" ref={menuRef} role="listbox" aria-label="选择模型" style={position}>
          <label className="model-search">
            <Search size={14} />
            <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索模型或供应商" autoFocus />
          </label>
          <div className="model-menu-list">
            {filtered.map((choice) => {
              const active = choice.providerId === selection?.providerId && choice.model === selection.model;
              return (
                <button
                  type="button"
                  role="option"
                  aria-selected={active}
                  className={active ? "model-menu-item active" : "model-menu-item"}
                  key={`${choice.providerId}-${choice.model}`}
                  onClick={() => handleSelect(choice)}
                >
                  <span className="model-menu-main"><Cpu size={14} /><strong>{choice.model}</strong></span>
                  <span className="model-provider">{choice.providerName}</span>
                  <Check size={14} className="model-check" />
                </button>
              );
            })}
            {filtered.length === 0 && <div className="model-menu-empty">没有匹配的模型</div>}
          </div>
        </div>,
        document.body
      )}
    </div>
  );
}
