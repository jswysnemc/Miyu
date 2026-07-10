import { Check, ChevronDown } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import "./select.css";

export type SelectOption<T extends string> = {
  value: T;
  label: string;
  description?: string;
};

type SelectProps<T extends string> = {
  value: T;
  options: SelectOption<T>[];
  disabled?: boolean;
  ariaLabel?: string;
  onChange: (value: T) => void;
};

/**
 * 渲染支持键盘导航的自定义单选下拉组件。
 *
 * @param props 当前值、选项和更新回调
 * @returns 自定义 combobox
 */
export function Select<T extends string>({ value, options, disabled, ariaLabel, onChange }: SelectProps<T>) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const current = options.find((option) => option.value === value) ?? options[0];

  useEffect(() => {
    const handlePointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, []);

  /** 处理方向键和选择操作。 */
  const handleKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>) => {
    if (!["ArrowDown", "ArrowUp", "Enter", " ", "Escape"].includes(event.key)) return;
    event.preventDefault();
    if (event.key === "Escape") {
      setOpen(false);
      return;
    }
    if (!open) {
      setOpen(true);
      return;
    }
    const index = Math.max(0, options.findIndex((option) => option.value === value));
    const nextIndex = event.key === "ArrowUp" ? Math.max(0, index - 1) : Math.min(options.length - 1, index + 1);
    if (event.key === "ArrowDown" || event.key === "ArrowUp") onChange(options[nextIndex].value);
    else setOpen(false);
  };

  return (
    <div className="ui-select" ref={rootRef}>
      <button type="button" role="combobox" aria-label={ariaLabel} aria-expanded={open} disabled={disabled} onClick={() => setOpen((visible) => !visible)} onKeyDown={handleKeyDown}>
        <span>{current?.label ?? value}</span><ChevronDown size={14} className={open ? "open" : ""} />
      </button>
      {open && (
        <div className="ui-select-menu" role="listbox">
          {options.map((option) => (
            <button type="button" role="option" aria-selected={option.value === value} className={option.value === value ? "active" : ""} key={option.value} onClick={() => { onChange(option.value); setOpen(false); }}>
              <span><strong>{option.label}</strong>{option.description && <small>{option.description}</small>}</span>
              <Check size={14} />
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
