type MiyuLogoProps = {
  size?: number;
};

/**
 * 渲染 Miyu 品牌标志：带猫耳轮廓的 M 字标与星点，呼应二次元桌面助手的定位。
 *
 * @param props 尺寸（像素，默认 20）
 * @returns 品牌 SVG 图标
 */
export function MiyuLogo({ size = 20 }: MiyuLogoProps) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" role="img" aria-label="Miyu">
      <rect x="1" y="1" width="30" height="30" rx="8.5" fill="var(--signal, #477d70)" />
      <path
        d="M8 23.5 V13.5 L12.4 8.6 L16 14.2 L19.6 8.6 L24 13.5 V23.5"
        fill="none"
        stroke="#f6faf8"
        strokeWidth="3"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <circle cx="25.4" cy="6.8" r="1.7" fill="#f6faf8" />
    </svg>
  );
}
