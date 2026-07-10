import { Image as ImageIcon, X } from "lucide-react";
import { useState } from "react";
import type { ComposerAttachment } from "./use-composer-attachments";
import { Modal } from "../../../shared/ui/dialog/modal";

/**
 * 渲染待发送图片缩略图和 token 标记。
 *
 * @param props 附件列表与删除回调
 * @returns 图片附件条
 */
export function AttachmentStrip({ attachments, onRemove }: { attachments: ComposerAttachment[]; onRemove: (id: number) => void }) {
  const [preview, setPreview] = useState<ComposerAttachment | null>(null);
  if (attachments.length === 0) return null;
  return (
    <>
      <div className="composer-attachments">
        {attachments.map((attachment) => (
          <div className="composer-attachment" key={attachment.id} title={attachment.name}>
            <button type="button" className="attachment-preview-button" onClick={() => setPreview(attachment)} aria-label={`预览 ${attachment.name}`}><img src={attachment.dataUrl} alt={attachment.name} /></button>
            <span className="attachment-name"><ImageIcon size={11} />{attachment.name}</span>
            <button type="button" className="attachment-remove" onClick={() => onRemove(attachment.id)} aria-label={`移除 ${attachment.name}`}><X size={13} /></button>
          </div>
        ))}
      </div>
      <Modal open={Boolean(preview)} title={preview?.name ?? "图片预览"} size="large" onClose={() => setPreview(null)}>
        {preview && <img className="attachment-preview-image" src={preview.dataUrl} alt={preview.name} />}
      </Modal>
    </>
  );
}
