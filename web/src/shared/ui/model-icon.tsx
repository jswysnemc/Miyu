type ModelIconProps = {
  model: string;
  size?: number;
};

type VendorStyle = {
  match: RegExp;
  label: string;
  background: string;
  color: string;
};

/** 模型名称到厂商徽标样式的映射表，按顺序取第一个命中的规则。 */
const VENDOR_STYLES: VendorStyle[] = [
  { match: /gpt|^o[0-9]|davinci|codex/i, label: "OA", background: "#10a37f", color: "#ffffff" },
  { match: /claude|anthropic|opus|sonnet|haiku/i, label: "An", background: "#d97757", color: "#ffffff" },
  { match: /gemini|palm|bard/i, label: "Ge", background: "#3d7ef2", color: "#ffffff" },
  { match: /deepseek/i, label: "DS", background: "#4d6bfe", color: "#ffffff" },
  { match: /qwen|tongyi/i, label: "Qw", background: "#615ced", color: "#ffffff" },
  { match: /glm|chatglm|zhipu/i, label: "GL", background: "#3859ff", color: "#ffffff" },
  { match: /llama/i, label: "Ll", background: "#0866ff", color: "#ffffff" },
  { match: /mistral|mixtral|magistral/i, label: "Mi", background: "#fa500f", color: "#ffffff" },
  { match: /grok/i, label: "Gr", background: "#1c1c1c", color: "#ffffff" },
  { match: /kimi|moonshot/i, label: "Ki", background: "#0d0d17", color: "#ffffff" },
  { match: /minimax|abab/i, label: "MM", background: "#f23f5d", color: "#ffffff" },
  { match: /doubao|seed/i, label: "Db", background: "#3b6dfd", color: "#ffffff" },
  { match: /hunyuan/i, label: "Hy", background: "#0052d9", color: "#ffffff" },
  { match: /command|cohere/i, label: "Co", background: "#39594d", color: "#ffffff" },
  { match: /ernie|wenxin/i, label: "Er", background: "#2932e1", color: "#ffffff" },
  { match: /step/i, label: "St", background: "#0057ff", color: "#ffffff" }
];

/**
 * 根据模型名称猜测厂商并渲染对应的圆角徽标图标。
 *
 * @param props 模型名称与尺寸（默认 16）
 * @returns 厂商徽标 SVG
 */
export function ModelIcon({ model, size = 16 }: ModelIconProps) {
  const vendor = VENDOR_STYLES.find((style) => style.match.test(model));
  const label = vendor?.label ?? model.slice(0, 2);
  const background = vendor?.background ?? "var(--graphite-soft, #394244)";
  const color = vendor?.color ?? "#eef2f0";
  return (
    <svg width={size} height={size} viewBox="0 0 20 20" role="img" aria-label={`模型 ${model}`}>
      <rect x="0.5" y="0.5" width="19" height="19" rx="5.5" fill={background} />
      <text
        x="10"
        y="10"
        textAnchor="middle"
        dominantBaseline="central"
        fill={color}
        fontFamily="var(--font-code, monospace)"
        fontSize="9"
        fontWeight="600"
      >
        {label}
      </text>
    </svg>
  );
}
