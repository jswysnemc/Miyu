import { ArrowUp, Check, Folder, GitBranch, HardDrive } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "../../api/client";
import { Modal } from "../../shared/ui/dialog/modal";

type ServerDirectoryDialogProps = {
  open: boolean;
  onClose: () => void;
  onSelect: (path: string) => Promise<void>;
};

/**
 * 渲染服务端目录浏览和工作区选择对话框。
 *
 * @param props 打开状态、关闭回调和目录选择回调
 * @returns 服务端目录选择弹层
 */
export function ServerDirectoryDialog({ open, onClose, onSelect }: ServerDirectoryDialogProps) {
  const [path, setPath] = useState<string | undefined>();
  const [selected, setSelected] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const listing = useQuery({ queryKey: ["workspace-directories", path], queryFn: () => api.workspaces.browse(path), enabled: open });

  /** 切换当前浏览目录。 */
  const navigate = (nextPath: string) => {
    setPath(nextPath);
    setSelected("");
  };

  /** 登记并切换到选中的服务端目录。 */
  const submit = async () => {
    const target = selected || listing.data?.current;
    if (!target) return;
    setSubmitting(true);
    try {
      await onSelect(target);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Modal
      open={open}
      title="打开服务端工作区"
      description="选择运行 Miyu Web 的服务器上的目录。浏览范围由服务端配置限制。"
      size="large"
      onClose={onClose}
      footer={<><button type="button" className="ui-button secondary" onClick={onClose}>取消</button><button type="button" className="ui-button primary" onClick={() => void submit()} disabled={submitting || !listing.data}>{submitting ? "正在打开" : "打开工作区"}</button></>}
    >
      <div className="server-directory-dialog">
        <aside className="directory-roots">
          <span>允许位置</span>
          {listing.data?.roots.map((root) => <button type="button" key={root.path} onClick={() => navigate(root.path)}><HardDrive size={14} /><span><strong>{root.name}</strong><small>{root.path}</small></span></button>)}
        </aside>
        <section className="directory-browser">
          <header><button type="button" onClick={() => listing.data?.parent && navigate(listing.data.parent)} disabled={!listing.data?.parent} aria-label="上级目录"><ArrowUp size={14} /></button><code>{listing.data?.current ?? "正在读取目录"}</code></header>
          <div className="directory-list">
            {listing.data?.entries.map((entry) => (
              <button type="button" className={selected === entry.path ? "selected" : ""} key={entry.path} onDoubleClick={() => navigate(entry.path)} onClick={() => setSelected(entry.path)}>
                <Folder size={16} /><span><strong>{entry.name}</strong><small>{entry.path}</small></span>{entry.git_repository && <span className="directory-git"><GitBranch size={12} />Git</span>}{selected === entry.path && <Check size={14} />}
              </button>
            ))}
            {listing.data?.entries.length === 0 && <div className="directory-empty">当前目录没有可浏览的子目录</div>}
            {listing.error && <div className="pane-error">{listing.error.message}</div>}
          </div>
        </section>
      </div>
    </Modal>
  );
}
