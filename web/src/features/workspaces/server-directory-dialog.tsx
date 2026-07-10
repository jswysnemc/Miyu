import { ArrowUp, Check, CornerDownLeft, Eye, EyeOff, Folder, GitBranch, HardDrive } from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { api } from "../../api/client";
import type { DirectoryEntry } from "../../api/contracts";
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
  const [pathDraft, setPathDraft] = useState("");
  const [selected, setSelected] = useState("");
  const [showHidden, setShowHidden] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const listing = useQuery({ queryKey: ["workspace-directories", path], queryFn: () => api.workspaces.browse(path), enabled: open });
  const entries = useMemo(() => sortEntries(listing.data?.entries ?? [], showHidden), [listing.data?.entries, showHidden]);
  const hiddenCount = (listing.data?.entries.length ?? 0) - sortEntries(listing.data?.entries ?? [], false).length;

  /** 切换当前浏览目录。 */
  const navigate = (nextPath: string) => {
    setPath(nextPath);
    setPathDraft("");
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
      footer={<><button type="button" className="ui-button secondary" onClick={onClose}>取消</button><button type="button" className="ui-button primary" onClick={() => void submit()} disabled={submitting || !listing.data}>{submitting ? "正在打开" : selected ? "打开选中目录" : "打开当前目录"}</button></>}
    >
      <div className="server-directory-dialog">
        <aside className="directory-roots">
          <span>允许位置</span>
          {listing.data?.roots.map((root) => <button type="button" key={root.path} onClick={() => navigate(root.path)}><HardDrive size={14} /><span><strong>{root.name}</strong><small>{root.path}</small></span></button>)}
        </aside>
        <section className="directory-browser">
          <header>
            <button type="button" onClick={() => listing.data?.parent && navigate(listing.data.parent)} disabled={!listing.data?.parent} aria-label="上级目录"><ArrowUp size={14} /></button>
            <input
              className="directory-path-input"
              value={pathDraft || listing.data?.current || ""}
              placeholder="输入服务器上的绝对路径后回车"
              spellCheck={false}
              onChange={(event) => setPathDraft(event.target.value)}
              onKeyDown={(event) => { if (event.key === "Enter" && pathDraft.trim()) navigate(pathDraft.trim()); }}
            />
            {pathDraft.trim() && <button type="button" onClick={() => navigate(pathDraft.trim())} aria-label="跳转到输入路径"><CornerDownLeft size={14} /></button>}
            <button type="button" onClick={() => setShowHidden((value) => !value)} aria-label={showHidden ? "隐藏点开头目录" : "显示点开头目录"}>
              {showHidden ? <EyeOff size={14} /> : <Eye size={14} />}
            </button>
          </header>
          <div className="directory-list">
            {entries.map((entry) => (
              <button type="button" className={selected === entry.path ? "selected" : ""} key={entry.path} onDoubleClick={() => navigate(entry.path)} onClick={() => setSelected(entry.path)}>
                <Folder size={16} /><span><strong>{entry.name}</strong><small>{entry.path}</small></span>{entry.git_repository && <span className="directory-git"><GitBranch size={12} />Git</span>}{selected === entry.path && <Check size={14} />}
              </button>
            ))}
            {entries.length === 0 && <div className="directory-empty">{hiddenCount > 0 ? `当前目录只有 ${hiddenCount} 个隐藏目录` : "当前目录没有可浏览的子目录"}</div>}
            {!showHidden && entries.length > 0 && hiddenCount > 0 && <div className="directory-hidden-hint">已折叠 {hiddenCount} 个点开头目录</div>}
            {listing.error && <div className="pane-error">{listing.error.message}</div>}
          </div>
        </section>
      </div>
    </Modal>
  );
}

/**
 * 过滤隐藏目录并把普通目录排在前面。
 *
 * @param entries 服务端目录条目
 * @param showHidden 是否显示点开头目录
 * @returns 排序后的目录条目
 */
function sortEntries(entries: DirectoryEntry[], showHidden: boolean): DirectoryEntry[] {
  const visible = showHidden ? entries : entries.filter((entry) => !entry.name.startsWith("."));
  return [...visible].sort((left, right) => {
    const leftHidden = left.name.startsWith(".") ? 1 : 0;
    const rightHidden = right.name.startsWith(".") ? 1 : 0;
    if (leftHidden !== rightHidden) return leftHidden - rightHidden;
    return left.name.localeCompare(right.name);
  });
}
