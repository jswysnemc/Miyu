import { BrainCircuit, Check, ChevronDown } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { ThinkingLevel } from "../../api/contracts";

const THINKING_OPTIONS: Array<{ value: ThinkingLevel; label: string; description: string }> = [
  { value: "auto", label: "自动", description: "由服务商按模型能力决定" },
  { value: "none", label: "关闭", description: "不请求额外推理" },
  { value: "low", label: "低", description: "更快响应，较少推理" },
  { value: "medium", label: "中", description: "速度与推理深度平衡" },
  { value: "high", label: "高", description: "适合复杂实现任务" },
  { value: "xhigh", label: "极高", description: "增加复杂问题推理预算" },
  { value: "max", label: "最大", description: "使用服务商支持的最高等级" }
];

type ThinkingSelectorProps = {
  value: ThinkingLevel;
  disabled: boolean;
  onChange: (value: ThinkingLevel) => void;
};

/**
 * 渲染向上展开的思考等级选择器。
 *
 * @param props 当前等级、禁用状态和更新回调
 * @returns 思考等级菜单
 */
export function ThinkingSelector({ value, disabled, onChange }: ThinkingSelectorProps) {
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ left: 12, bottom: 56, width: 290 });
  const current = THINKING_OPTIONS.find((option) => option.value === value) ?? THINKING_OPTIONS[0];

  useEffect(() => {
    /** 在思考等级菜单外点击时关闭菜单。 */
    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!triggerRef.current?.contains(target) && !menuRef.current?.contains(target)) setOpen(false);
    };
    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, []);

  useEffect(() => {
    if (!open) return;

    /** 根据触发按钮计算菜单固定坐标。 */
    const updatePosition = () => {
      const rect = triggerRef.current?.getBoundingClientRect();
      if (!rect) return;
      const padding = 12;
      const width = Math.min(310, window.innerWidth - padding * 2);
      const left = Math.max(padding, Math.min(rect.left, window.innerWidth - width - padding));
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
   * 更新思考等级并关闭菜单。
   *
   * @param level 新思考等级
   */
  const selectLevel = (level: ThinkingLevel) => {
    onChange(level);
    setOpen(false);
  };

  return (
    <div className="thinking-selector">
      <button
        ref={triggerRef}
        type="button"
        className="thinking-selector-trigger"
        onClick={() => setOpen((currentOpen) => !currentOpen)}
        disabled={disabled}
        aria-haspopup="listbox"
        aria-expanded={open}
      >
        <BrainCircuit size={14} />
        <span>思考 {current.label}</span>
        <ChevronDown size={12} className={open ? "thinking-chevron open" : "thinking-chevron"} />
      </button>
      {open && createPortal(
        <div className="thinking-menu" ref={menuRef} role="listbox" aria-label="选择思考等级" style={position}>
          <div className="thinking-menu-head">思考等级</div>
          <div className="thinking-menu-list">
            {THINKING_OPTIONS.map((option) => (
              <button
                type="button"
                role="option"
                aria-selected={option.value === value}
                className={option.value === value ? "thinking-menu-item active" : "thinking-menu-item"}
                onClick={() => selectLevel(option.value)}
                key={option.value}
              >
                <span><strong>{option.label}</strong><small>{option.description}</small></span>
                <Check size={14} />
              </button>
            ))}
          </div>
        </div>,
        document.body
      )}
    </div>
  );
}
