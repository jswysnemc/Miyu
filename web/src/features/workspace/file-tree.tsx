import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, ChevronRight, ChevronsLeft, ChevronsRight, FilePlus2, Folder, FolderOpen, FolderPlus, Pencil, RefreshCw, Trash2, X } from "lucide-react";
import { useState } from "react";
import { api } from "../../api/client";
import type { FileNode } from "../../api/contracts";
import { useConfirm } from "../../shared/ui/dialog/dialog-provider";
import { FileTypeIcon } from "../../shared/ui/file-icon";

type FileTreeProps = {
  selectedFile: string | null;
  onSelectFile: (path: string) => void;
  onClearFile: () => void;
};

type FileAction = { kind: "file" | "directory" | "rename"; value: string } | null;

/**
 * 渲染支持创建、重命名和删除的工作区文件树。
 *
 * @param props 当前文件选择和更新回调
 * @returns 文件浏览器
 */
export function FileTree({ selectedFile, onSelectFile, onClearFile }: FileTreeProps) {
  const confirm = useConfirm();
  const queryClient = useQueryClient();
  const tree = useQuery({ queryKey: ["file-tree"], queryFn: api.workspace.tree, refetchOnWindowFocus: true, refetchInterval: 15_000 });
  const [focusedPath, setFocusedPath] = useState<string | null>(selectedFile);
  const [action, setAction] = useState<FileAction>(null);
  const [collapsed, setCollapsed] = useState(false);
  const [error, setError] = useState("");
  const focusedNode = findNode(tree.data ?? [], focusedPath);

  /** 打开新建文件或目录输入栏。 */
  const beginCreate = (kind: "file" | "directory") => {
    const parent = focusedNode?.kind === "directory" ? focusedNode.path : parentPath(focusedPath ?? "");
    setAction({ kind, value: parent ? `${parent}/` : "" });
    setError("");
  };

  /** 打开重命名输入栏。 */
  const beginRename = () => {
    if (!focusedPath) return;
    setAction({ kind: "rename", value: focusedPath });
    setError("");
  };

  /** 提交当前文件操作。 */
  const submitAction = async () => {
    if (!action?.value.trim()) return;
    setError("");
    try {
      if (action.kind === "rename" && focusedPath) {
        const entry = await api.workspace.rename(focusedPath, action.value.trim());
        if (selectedFile === focusedPath) onSelectFile(entry.path);
        setFocusedPath(entry.path);
      } else if (action.kind !== "rename") {
        const entry = await api.workspace.create(action.value.trim(), action.kind);
        setFocusedPath(entry.path);
        if (entry.kind === "file") onSelectFile(entry.path);
      }
      setAction(null);
      await refreshWorkspaceQueries(queryClient);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  /** 删除当前聚焦的文件或目录。 */
  const deleteFocused = async () => {
    if (!focusedPath) return;
    const confirmed = await confirm({
      title: "删除工作区条目",
      description: `将删除“${focusedPath}”${focusedNode?.kind === "directory" ? "及目录中的全部内容" : ""}。`,
      confirmLabel: "删除",
      danger: true
    });
    if (!confirmed) return;
    setError("");
    try {
      await api.workspace.remove(focusedPath);
      if (selectedFile === focusedPath || selectedFile?.startsWith(`${focusedPath}/`)) onClearFile();
      setFocusedPath(null);
      await refreshWorkspaceQueries(queryClient);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  if (collapsed) {
    return (
      <aside className="file-tree collapsed">
        <button type="button" className="file-tree-expand" onClick={() => setCollapsed(false)} aria-label="展开文件树"><ChevronsRight size={14} /></button>
      </aside>
    );
  }

  return (
    <aside className="file-tree">
      <div className="file-tree-head">
        <span>文件</span>
        <div className="file-tree-actions">
          <button type="button" onClick={() => beginCreate("file")} aria-label="新建文件"><FilePlus2 size={13} /></button>
          <button type="button" onClick={() => beginCreate("directory")} aria-label="新建目录"><FolderPlus size={13} /></button>
          <button type="button" onClick={beginRename} disabled={!focusedPath} aria-label="重命名"><Pencil size={12} /></button>
          <button type="button" onClick={() => void deleteFocused()} disabled={!focusedPath} aria-label="删除"><Trash2 size={12} /></button>
          <button type="button" onClick={() => void tree.refetch()} aria-label="刷新文件树"><RefreshCw size={12} /></button>
          <button type="button" onClick={() => setCollapsed(true)} aria-label="折叠文件树"><ChevronsLeft size={12} /></button>
        </div>
      </div>
      <div className="file-tree-scroll">
        {action && (
          <div className="file-action-bar">
            {action.kind === "directory" ? <FolderPlus size={13} /> : action.kind === "file" ? <FilePlus2 size={13} /> : <Pencil size={13} />}
            <input autoFocus value={action.value} onChange={(event) => setAction({ ...action, value: event.target.value })} onKeyDown={(event) => { if (event.key === "Enter") void submitAction(); if (event.key === "Escape") setAction(null); }} spellCheck={false} />
            <button type="button" onClick={() => void submitAction()} aria-label="确认"><Check size={12} /></button>
            <button type="button" onClick={() => setAction(null)} aria-label="取消"><X size={12} /></button>
          </div>
        )}
        {tree.data?.map((node) => <TreeNode key={node.path} node={node} selectedFile={selectedFile} focusedPath={focusedPath} onFocus={setFocusedPath} onSelectFile={onSelectFile} depth={0} />)}
        {(tree.error || error) && <p className="pane-error">{error || tree.error?.message}</p>}
      </div>
    </aside>
  );
}

/** 渲染单个递归文件树节点。 */
function TreeNode({ node, selectedFile, focusedPath, onFocus, onSelectFile, depth }: { node: FileNode; selectedFile: string | null; focusedPath: string | null; onFocus: (path: string) => void; onSelectFile: (path: string) => void; depth: number }) {
  const [open, setOpen] = useState(depth < 1);
  const directory = node.kind === "directory";
  const active = selectedFile === node.path || focusedPath === node.path;
  return (
    <div>
      <button
        type="button"
        className={active ? "tree-row active" : "tree-row"}
        style={{ paddingLeft: `${8 + depth * 13}px` }}
        onClick={() => {
          onFocus(node.path);
          if (directory) setOpen((value) => !value);
          else onSelectFile(node.path);
        }}
      >
        {directory ? <ChevronRight size={12} className={open ? "tree-chevron open" : "tree-chevron"} /> : <span className="tree-spacer" />}
        {directory ? (open ? <FolderOpen size={14} /> : <Folder size={14} />) : <FileTypeIcon name={node.name} size={13} />}
        <span>{node.name}</span>
      </button>
      {directory && open && node.children.map((child) => <TreeNode key={child.path} node={child} selectedFile={selectedFile} focusedPath={focusedPath} onFocus={onFocus} onSelectFile={onSelectFile} depth={depth + 1} />)}
    </div>
  );
}

/** 返回树中指定路径的节点。 */
function findNode(nodes: FileNode[], path: string | null): FileNode | null {
  if (!path) return null;
  for (const node of nodes) {
    if (node.path === path) return node;
    const child = findNode(node.children, path);
    if (child) return child;
  }
  return null;
}

/** 返回相对路径的父目录。 */
function parentPath(path: string): string {
  const index = path.lastIndexOf("/");
  return index < 0 ? "" : path.slice(0, index);
}

/** 刷新文件树、文件内容和 Git 状态。 */
async function refreshWorkspaceQueries(queryClient: ReturnType<typeof useQueryClient>): Promise<void> {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: ["file-tree"] }),
    queryClient.invalidateQueries({ queryKey: ["file"] }),
    queryClient.invalidateQueries({ queryKey: ["workspace-diff"] })
  ]);
}
