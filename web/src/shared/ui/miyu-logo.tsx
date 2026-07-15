type MiyuLogoProps = {
  size?: number;
};

/**
 * 渲染 Miyu 品牌标志：圆角方块上的几何 M，带轻微猫耳提示。
 *
 * @param props 尺寸（像素，默认 20）
 * @returns 品牌 SVG 图标
 */
export function MiyuLogo({ size = 20 }: MiyuLogoProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" role="img" aria-label="Miyu">
      <defs>
        <linearGradient id="miyu-mark" x1="6" y1="4" x2="28" y2="30" gradientUnits="userSpaceOnUse">
          <stop offset="0%" stopColor="color-mix(in srgb, var(--signal, #477d70) 88%, #ffffff)" />
          <stop offset="100%" stopColor="var(--signal, #477d70)" />
        </linearGradient>
      </defs>
      <path
        d="M8.2 4.2 L12.4 1.8 L16 5.1 L19.6 1.8 L23.8 4.2 L28 8.6 V24.8 C28 27 26.2 28.8 24 28.8 H8 C5.8 28.8 4 27 4 24.8 V8.6 Z"
        fill="url(#miyu-mark)"
      />
      <path
        d="M9.2 22.4 V13.1 L13.4 8.8 L16 13.1 L18.6 8.8 L22.8 13.1 V22.4"
        fill="none"
        stroke="#f7fbf9"
        strokeWidth="2.4"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <circle cx="24.7" cy="8.2" r="1.35" fill="#f7fbf9" opacity="0.92" />
    </svg>
  );
}
