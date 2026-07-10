import Editor, { loader } from "@monaco-editor/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Save } from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../../api/client";
import { useTheme } from "../theme/theme";

export function EditorPane({ path }: { path: string | null }) {
  const { theme } = useTheme();
  const queryClient = useQueryClient();
  const file = useQuery({ queryKey: ["file", path], queryFn: () => api.workspace.file(path!), enabled: Boolean(path) });
  const [content, setContent] = useState("");
  const [externalChange, setExternalChange] = useState(false);
  const [editorReady, setEditorReady] = useState(false);
  useEffect(() => {
    let active = true;
    import("monaco-editor").then((monaco) => {
      loader.config({ monaco });
      if (active) setEditorReady(true);
    });
    return () => { active = false; };
  }, []);
  useEffect(() => {
    setContent("");
    setExternalChange(false);
  }, [path]);
  useEffect(() => {
    if (!file.data) return;
    setContent((current) => {
      const dirty = current !== "" && current !== file.data.content;
      if (dirty) {
        setExternalChange(true);
        return current;
      }
      setExternalChange(false);
      return file.data.content;
    });
  }, [file.data]);
  const save = useMutation({
    mutationFn: () => api.workspace.save(path!, content, file.data?.modified_at),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["file", path] });
      await queryClient.invalidateQueries({ queryKey: ["workspace-diff"] });
      setExternalChange(false);
    }
  });
  if (!path) return <div className="editor-empty"><FileCodePlaceholder /><p>从文件树选择文本文件</p></div>;
  return (
    <section className="editor-pane">
      <header className="editor-head">
        <span title={path}>{path}</span>
        {externalChange && <span className="editor-external-change">磁盘内容已变化</span>}
        <button type="button" className="editor-save" onClick={() => save.mutate()} disabled={!file.data || content === file.data.content || save.isPending}>
          <Save size={14} /> 保存
        </button>
      </header>
      <div className="editor-area">
        {file.data && editorReady && (
          <Editor
            path={path}
            language={languageForPath(path)}
            value={content}
            onChange={(value) => setContent(value ?? "")}
            theme={theme === "graphite" || theme === "ocean" || (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches) ? "vs-dark" : "light"}
            options={{ minimap: { enabled: false }, fontFamily: "Fira Code", fontSize: 13, lineHeight: 21, padding: { top: 12 }, automaticLayout: true, scrollBeyondLastLine: false }}
          />
        )}
        {(file.isLoading || !editorReady) && <div className="editor-state">加载编辑器</div>}
        {file.error && <div className="pane-error">{file.error.message}</div>}
        {save.error && <div className="pane-error">{save.error.message}</div>}
      </div>
    </section>
  );
}

function FileCodePlaceholder() {
  return <div className="file-code-placeholder">&lt;/&gt;</div>;
}

function languageForPath(path: string) {
  const extension = path.split(".").pop()?.toLowerCase();
  return ({ rs: "rust", ts: "typescript", tsx: "typescript", js: "javascript", jsx: "javascript", json: "json", md: "markdown", css: "css", html: "html", py: "python", go: "go", sh: "shell", toml: "ini", yaml: "yaml", yml: "yaml" } as Record<string, string>)[extension ?? ""] ?? "plaintext";
}
