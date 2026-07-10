use super::manager::TerminalInfo;
use anyhow::{Context, Result};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::broadcast;

const TERMINAL_BROADCAST_CAPACITY: usize = 256;
const TERMINAL_REPLAY_BYTES: usize = 1024 * 1024;

/// 单个 PTY 会话及其输入输出通道。
pub(crate) struct TerminalSession {
    id: String,
    title: String,
    size: Arc<RwLock<(u16, u16)>>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    child: Mutex<Box<dyn Child + Send>>,
    output: broadcast::Sender<Vec<u8>>,
    replay: Arc<Mutex<VecDeque<u8>>>,
}

impl TerminalSession {
    /// 在指定目录启动用户 Shell。
    ///
    /// 参数:
    /// - `id`: 终端 ID
    /// - `cwd`: 工作目录
    /// - `cols`: 初始列数
    /// - `rows`: 初始行数
    ///
    /// 返回:
    /// - PTY 会话
    pub(super) fn spawn(id: String, cwd: &Path, cols: u16, rows: u16) -> Result<Self> {
        let pair = native_pty_system().openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut command = CommandBuilder::new(&shell);
        command.cwd(cwd);
        let child = pair
            .slave
            .spawn_command(command)
            .with_context(|| format!("failed to start terminal shell {shell}"))?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let (output, _) = broadcast::channel(TERMINAL_BROADCAST_CAPACITY);
        let output_thread = output.clone();
        let replay = Arc::new(Mutex::new(VecDeque::new()));
        let replay_thread = replay.clone();
        std::thread::Builder::new()
            .name(format!("miyu-terminal-{id}"))
            .spawn(move || read_terminal_output(&mut reader, &output_thread, &replay_thread))?;
        Ok(Self {
            title: cwd
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("terminal")
                .to_string(),
            id,
            size: Arc::new(RwLock::new((cols, rows))),
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
            child: Mutex::new(child),
            output,
            replay,
        })
    }

    /// 返回终端摘要。
    pub(super) fn info(&self) -> TerminalInfo {
        let (cols, rows) = self.size.read().map(|size| *size).unwrap_or((80, 24));
        TerminalInfo {
            id: self.id.clone(),
            title: self.title.clone(),
            cols,
            rows,
        }
    }

    /// 订阅终端输出。
    pub(super) fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.output.subscribe()
    }

    /// 返回连接前已经产生的终端输出。
    pub(super) fn replay(&self) -> Vec<u8> {
        self.replay
            .lock()
            .map(|buffer| buffer.iter().copied().collect())
            .unwrap_or_default()
    }

    /// 写入终端输入。
    ///
    /// 参数:
    /// - `bytes`: 原始输入字节
    pub(super) fn write(&self, bytes: &[u8]) -> Result<()> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| anyhow::anyhow!("terminal writer lock is poisoned"))?;
        writer.write_all(bytes)?;
        writer.flush()?;
        Ok(())
    }

    /// 调整终端尺寸。
    ///
    /// 参数:
    /// - `cols`: 新列数
    /// - `rows`: 新行数
    pub(super) fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let cols = cols.max(1);
        let rows = rows.max(1);
        self.master
            .lock()
            .map_err(|_| anyhow::anyhow!("terminal master lock is poisoned"))?
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })?;
        if let Ok(mut size) = self.size.write() {
            *size = (cols, rows);
        }
        Ok(())
    }

    /// 终止终端子进程。
    pub(super) fn kill(&self) -> Result<()> {
        self.child
            .lock()
            .map_err(|_| anyhow::anyhow!("terminal child lock is poisoned"))?
            .kill()?;
        Ok(())
    }
}

/// 持续读取 PTY 输出并广播给浏览器。
fn read_terminal_output(
    reader: &mut Box<dyn Read + Send>,
    output: &broadcast::Sender<Vec<u8>>,
    replay: &Arc<Mutex<VecDeque<u8>>>,
) {
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                let chunk = buffer[..read].to_vec();
                if let Ok(mut replay) = replay.lock() {
                    replay.extend(chunk.iter().copied());
                    while replay.len() > TERMINAL_REPLAY_BYTES {
                        replay.pop_front();
                    }
                }
                let _ = output.send(chunk);
            }
        }
    }
}
