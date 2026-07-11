export type AnchorRect = {
  left: number;
  right: number;
  bottom: number;
};

export type AnchoredPopoverOptions = {
  viewportWidth: number;
  preferredWidth: number;
  minimumWidth: number;
  align: "left" | "right";
  padding?: number;
  gap?: number;
};

export type AnchoredPopoverPosition = {
  left: number;
  top: number;
  width: number;
};

/**
 * 计算固定定位弹层的视口内坐标。
 *
 * @param anchor 触发器的视口坐标
 * @param options 视口宽度、菜单宽度和对齐方式
 * @returns 弹层左侧、顶部和宽度
 */
export function calculateAnchoredPopoverPosition(
  anchor: AnchorRect,
  options: AnchoredPopoverOptions
): AnchoredPopoverPosition {
  const padding = options.padding ?? 12;
  const gap = options.gap ?? 6;
  const availableWidth = Math.max(0, options.viewportWidth - padding * 2);
  const minimumWidth = Math.min(Math.max(options.minimumWidth, 0), availableWidth);
  const width = Math.min(Math.max(options.preferredWidth, minimumWidth), availableWidth);
  const preferredLeft = options.align === "right" ? anchor.right - width : anchor.left;
  const left = Math.max(padding, Math.min(preferredLeft, options.viewportWidth - width - padding));
  return { left, top: anchor.bottom + gap, width };
}
