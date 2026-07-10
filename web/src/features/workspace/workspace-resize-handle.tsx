import type { PointerEvent as ReactPointerEvent } from "react";

type WorkspaceResizeHandleProps = {
  onResize: (width: number) => void;
};

/**
 * 渲染右侧工作区拖动手柄，并把指针位置转换为面板宽度。
 *
 * @param props 调整宽度回调
 * @returns 工作区拖动手柄
 */
export function WorkspaceResizeHandle({ onResize }: WorkspaceResizeHandleProps) {
  /**
   * 开始监听全局指针移动，直到用户释放指针。
   *
   * @param event 手柄指针按下事件
   */
  const handlePointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    document.body.classList.add("workspace-resizing");

    // 1. 按指针到视口右侧的距离计算工作区宽度
    const handlePointerMove = (moveEvent: PointerEvent) => {
      onResize(window.innerWidth - moveEvent.clientX);
    };

    // 2. 释放指针后统一清理全局监听器
    const handlePointerUp = () => {
      document.body.classList.remove("workspace-resizing");
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp, { once: true });
  };

  return (
    <div
      className="workspace-resize-handle"
      role="separator"
      aria-label="调整工作区宽度"
      aria-orientation="vertical"
      onPointerDown={handlePointerDown}
    >
      <span />
    </div>
  );
}
