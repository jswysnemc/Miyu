import { X } from "lucide-react";
import type { ComposerAttachment } from "./use-composer-attachments";

/**
 * 渲染待发送图片缩略图和 token 标记。
 *
 * @param props 附件列表与删除回调
 * @returns 图片附件条
 */
export function AttachmentStrip({ attachments, onRemove }: { attachments: ComposerAttachment[]; onRemove: (id: number) => void }) {
  if (attachments.length === 0) return null;
  return (
    <div className="composer-attachments">
      {attachments.map((attachment) => (
        <div className="composer-attachment" key={attachment.id} title={`${attachment.token} ${attachment.name}`}>
          <img src={attachment.dataUrl} alt={attachment.token} />
          <span className="attachment-token">{attachment.token}</span>
          <button type="button" onClick={() => onRemove(attachment.id)} aria-label={`移除 ${attachment.name}`}><X size={13} /></button>
        </div>
      ))}
    </div>
  );
}
