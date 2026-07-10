import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { useEffect, useRef, useState } from "react";
import { api } from "../../api/client";

export function TerminalPane() {
  const containerRef = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    let disposed = false;
    let socket: WebSocket | null = null;
    let terminalId: string | null = null;
    const terminal = new Terminal({ fontFamily: "Fira Code", fontSize: 12, lineHeight: 1.25, cursorBlink: true, theme: terminalTheme() });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(container);
    fit.fit();
    const start = async () => {
      const listed = await api.terminals.list();
      const info = listed.terminals[0] ?? await api.terminals.create(terminal.cols, terminal.rows);
      if (disposed) {
        if (!listed.terminals[0]) await api.terminals.remove(info.id);
        return;
      }
      terminalId = info.id;
      const protocol = location.protocol === "https:" ? "wss:" : "ws:";
      socket = new WebSocket(`${protocol}//${location.host}/api/terminals/${info.id}/socket`);
      socket.binaryType = "arraybuffer";
      socket.onmessage = (event) => {
        if (event.data instanceof ArrayBuffer) terminal.write(new Uint8Array(event.data));
        else terminal.write(String(event.data));
      };
      socket.onerror = () => setError("终端连接失败");
      terminal.onData((data) => { if (socket?.readyState === WebSocket.OPEN) socket.send(new TextEncoder().encode(data)); });
      socket.onopen = () => socket?.send(JSON.stringify({ type: "resize", cols: terminal.cols, rows: terminal.rows }));
    };
    void start().catch((reason: unknown) => setError(reason instanceof Error ? reason.message : String(reason)));
    const observer = new ResizeObserver(() => {
      fit.fit();
      if (socket?.readyState === WebSocket.OPEN) socket.send(JSON.stringify({ type: "resize", cols: terminal.cols, rows: terminal.rows }));
    });
    observer.observe(container);
    return () => {
      disposed = true;
      observer.disconnect();
      socket?.close();
      terminal.dispose();
      if (terminalId) void api.terminals.remove(terminalId);
    };
  }, []);
  return <section className="terminal-pane"><div className="terminal-surface" ref={containerRef} />{error && <div className="pane-error terminal-error">{error}</div>}</section>;
}

function terminalTheme() {
  const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  return dark
    ? { background: "#111512", foreground: "#e5e9e6", cursor: "#52c488", selectionBackground: "#315944" }
    : { background: "#202723", foreground: "#eef3ef", cursor: "#67d496", selectionBackground: "#456052" };
}
