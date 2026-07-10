import { useQuery } from "@tanstack/react-query";
import { ChevronRight, File, Folder, FolderOpen, RefreshCw } from "lucide-react";
import { useState } from "react";
import { api } from "../../api/client";
import type { FileNode } from "../../api/contracts";

export function FileTree({ selectedFile, onSelectFile }: { selectedFile: string | null; onSelectFile: (path: string) => void }) {
  const tree = useQuery({ queryKey: ["file-tree"], queryFn: api.workspace.tree });
  return (
    <aside className="file-tree">
      <div className="file-tree-head"><span>Explorer</span><button type="button" className="icon-button" onClick={() => void tree.refetch()} aria-label="刷新文件树"><RefreshCw size={13} /></button></div>
      <div className="file-tree-scroll">
        {tree.data?.map((node) => <TreeNode key={node.path} node={node} selectedFile={selectedFile} onSelectFile={onSelectFile} depth={0} />)}
        {tree.error && <p className="pane-error">{tree.error.message}</p>}
      </div>
    </aside>
  );
}

function TreeNode({ node, selectedFile, onSelectFile, depth }: { node: FileNode; selectedFile: string | null; onSelectFile: (path: string) => void; depth: number }) {
  const [open, setOpen] = useState(depth < 1);
  const directory = node.kind === "directory";
  return (
    <div>
      <button
        type="button"
        className={selectedFile === node.path ? "tree-row active" : "tree-row"}
        style={{ paddingLeft: `${8 + depth * 13}px` }}
        onClick={() => directory ? setOpen((value) => !value) : onSelectFile(node.path)}
      >
        {directory ? <ChevronRight size={12} className={open ? "tree-chevron open" : "tree-chevron"} /> : <span className="tree-spacer" />}
        {directory ? (open ? <FolderOpen size={14} /> : <Folder size={14} />) : <File size={13} />}
        <span>{node.name}</span>
      </button>
      {directory && open && node.children.map((child) => <TreeNode key={child.path} node={child} selectedFile={selectedFile} onSelectFile={onSelectFile} depth={depth + 1} />)}
    </div>
  );
}
