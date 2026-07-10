import type { IDisposable, Terminal } from "@xterm/xterm";

export type TerminalConnectionStatus = "connecting" | "connected" | "reconnecting" | "failed";

type TerminalSessionControllerOptions = {
  terminalId: string;
  terminal: Terminal;
  onStatusChange: (status: TerminalConnectionStatus) => void;
  onError: (message: string | null) => void;
};

export type TerminalSessionController = {
  resize: (cols: number, rows: number) => void;
  dispose: () => void;
};

const MAX_RECONNECT_ATTEMPTS = 5;

/**
 * 建立可重连的终端 WebSocket，并转发 xterm 输入与尺寸变化。
 *
 * @param options 终端标识、xterm 实例和状态回调
 * @returns 终端连接控制器
 */
export function connectTerminalSession(options: TerminalSessionControllerOptions): TerminalSessionController {
  let socket: WebSocket | null = null;
  let disposed = false;
  let reconnectAttempts = 0;
  let reconnectTimer: number | null = null;
  let lastSize = { cols: options.terminal.cols, rows: options.terminal.rows };
  const inputSubscriptions: IDisposable[] = [];

  /** 向已连接 WebSocket 写入终端字节。 */
  const sendBytes = (bytes: Uint8Array) => {
    if (socket?.readyState === WebSocket.OPEN) socket.send(bytes);
  };

  /** 发送最近一次终端尺寸。 */
  const sendResize = () => {
    if (socket?.readyState !== WebSocket.OPEN) return;
    socket.send(JSON.stringify({ type: "resize", cols: lastSize.cols, rows: lastSize.rows }));
  };

  /** 安排下一次连接重试。 */
  const scheduleReconnect = () => {
    if (disposed) return;
    reconnectAttempts += 1;
    if (reconnectAttempts > MAX_RECONNECT_ATTEMPTS) {
      options.onStatusChange("failed");
      options.onError("终端连接已断开，请重新选择终端或新建会话");
      return;
    }
    options.onStatusChange("reconnecting");
    const delay = Math.min(3_000, 300 * 2 ** (reconnectAttempts - 1));
    reconnectTimer = window.setTimeout(connect, delay);
  };

  /** 建立一次 WebSocket 连接。 */
  const connect = () => {
    if (disposed) return;
    options.onStatusChange(reconnectAttempts === 0 ? "connecting" : "reconnecting");
    const protocol = location.protocol === "https:" ? "wss:" : "ws:";
    const nextSocket = new WebSocket(`${protocol}//${location.host}/api/terminals/${options.terminalId}/socket`);
    nextSocket.binaryType = "arraybuffer";
    socket = nextSocket;
    nextSocket.onopen = () => {
      if (disposed || socket !== nextSocket) return;
      if (reconnectAttempts > 0) options.terminal.reset();
      reconnectAttempts = 0;
      options.onError(null);
      options.onStatusChange("connected");
      sendResize();
      options.terminal.focus();
    };
    nextSocket.onmessage = (event) => {
      if (event.data instanceof ArrayBuffer) options.terminal.write(new Uint8Array(event.data));
      else options.terminal.write(String(event.data));
    };
    nextSocket.onerror = () => nextSocket.close();
    nextSocket.onclose = () => {
      if (socket === nextSocket) socket = null;
      scheduleReconnect();
    };
  };

  inputSubscriptions.push(options.terminal.onData((data) => sendBytes(new TextEncoder().encode(data))));
  inputSubscriptions.push(options.terminal.onBinary((data) => sendBytes(Uint8Array.from(data, (character) => character.charCodeAt(0)))));
  connect();

  return {
    resize(cols, rows) {
      lastSize = { cols: Math.max(1, cols), rows: Math.max(1, rows) };
      sendResize();
    },
    dispose() {
      disposed = true;
      if (reconnectTimer !== null) window.clearTimeout(reconnectTimer);
      for (const subscription of inputSubscriptions) subscription.dispose();
      socket?.close();
      socket = null;
    }
  };
}
