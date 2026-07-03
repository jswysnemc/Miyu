use anyhow::{bail, Result};
use std::io::ErrorKind;
use std::process::{Output, Stdio};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// 用 shell 执行短命令并在超时时清理进程。
///
/// 参数:
/// - `command`: shell 命令文本
/// - `timeout_seconds`: 超时时间，单位秒
///
/// 返回:
/// - 命令输出
pub(crate) async fn run_shell_command(command: &str, timeout_seconds: u64) -> Result<Output> {
    let duration = Duration::from_secs(timeout_seconds.max(1));
    let mut missing = Vec::new();
    for (program, mut shell) in shell_commands(command) {
        match run_command_with_timeout(&mut shell, duration).await {
            Ok(output) => return Ok(output),
            Err(CommandRunError::NotFound) => missing.push(program),
            Err(CommandRunError::Timeout) => {
                bail!("shell command timed out after {timeout_seconds}s")
            }
            Err(CommandRunError::Other(err)) => return Err(err),
        }
    }
    bail!("no supported shell found; tried {}", missing.join(", "))
}

/// 启动后台 shell 命令。
///
/// 参数:
/// - `command`: shell 命令文本
/// - `cwd`: 工作目录
/// - `stdout`: stdout 重定向
/// - `stderr`: stderr 重定向
///
/// 返回:
/// - 启动后的进程信息
pub(crate) fn spawn_background_shell(
    command: &str,
    cwd: &std::path::Path,
    stdout: std::fs::File,
    stderr: std::fs::File,
) -> Result<BackgroundProcess> {
    let mut shell = shell_command(command);
    configure_process_group(&mut shell);
    let child = shell
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .kill_on_drop(false)
        .spawn()?;
    let pid = child
        .id()
        .ok_or_else(|| anyhow::anyhow!("background process id is unavailable"))?;
    drop(child);
    Ok(BackgroundProcess {
        pid,
        pgid: process_group_id(pid),
    })
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BackgroundProcess {
    pub(crate) pid: u32,
    pub(crate) pgid: Option<i32>,
}

enum CommandRunError {
    NotFound,
    Timeout,
    Other(anyhow::Error),
}

/// 执行命令并在超时时终止进程组。
///
/// 参数:
/// - `command`: 已配置的命令
/// - `duration`: 超时时间
///
/// 返回:
/// - 命令输出
async fn run_command_with_timeout(
    command: &mut Command,
    duration: Duration,
) -> std::result::Result<Output, CommandRunError> {
    configure_process_group(command);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|err| {
        if err.kind() == ErrorKind::NotFound {
            CommandRunError::NotFound
        } else {
            CommandRunError::Other(err.into())
        }
    })?;
    let pid = child.id();
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let stdout_task = tokio::spawn(async move { read_pipe(&mut stdout).await });
    let stderr_task = tokio::spawn(async move { read_pipe(&mut stderr).await });
    let status = match tokio::time::timeout(duration, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(err)) => return Err(CommandRunError::Other(err.into())),
        Err(_) => {
            if let Some(pid) = pid {
                terminate_process(pid, process_group_id(pid), true).await;
            } else {
                let _ = child.kill().await;
            }
            return Err(CommandRunError::Timeout);
        }
    };
    let stdout = stdout_task
        .await
        .map_err(|err| CommandRunError::Other(err.into()))?
        .map_err(|err| CommandRunError::Other(err.into()))?;
    let stderr = stderr_task
        .await
        .map_err(|err| CommandRunError::Other(err.into()))?
        .map_err(|err| CommandRunError::Other(err.into()))?;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// 读取子进程管道内容。
///
/// 参数:
/// - `pipe`: 可选管道
///
/// 返回:
/// - 读取到的字节
async fn read_pipe<T>(pipe: &mut Option<T>) -> std::io::Result<Vec<u8>>
where
    T: tokio::io::AsyncRead + Unpin,
{
    let mut buffer = Vec::new();
    if let Some(pipe) = pipe {
        pipe.read_to_end(&mut buffer).await?;
    }
    Ok(buffer)
}

#[cfg(windows)]
fn shell_commands(command: &str) -> Vec<(&'static str, Command)> {
    let mut pwsh = Command::new("pwsh");
    pwsh.arg("-NoLogo")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(command);

    let mut powershell = Command::new("powershell");
    powershell
        .arg("-NoLogo")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(command);

    let mut cmd = Command::new("cmd");
    cmd.arg("/S").arg("/C").arg(command);

    vec![("pwsh", pwsh), ("powershell", powershell), ("cmd", cmd)]
}

#[cfg(not(windows))]
fn shell_commands(command: &str) -> Vec<(&'static str, Command)> {
    vec![("sh", shell_command(command))]
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut shell = Command::new("cmd");
    shell.arg("/S").arg("/C").arg(command);
    shell
}

#[cfg(not(windows))]
fn shell_command(command: &str) -> Command {
    let mut shell = Command::new("sh");
    shell.arg("-lc").arg(command);
    shell
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

#[cfg(unix)]
fn process_group_id(pid: u32) -> Option<i32> {
    Some(pid as i32)
}

#[cfg(not(unix))]
fn process_group_id(_pid: u32) -> Option<i32> {
    None
}

/// 判断进程是否仍存在。
///
/// 参数:
/// - `pid`: 进程 ID
///
/// 返回:
/// - 是否存在
pub(crate) fn process_exists(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        false
    }
}

/// 终止进程或进程组。
///
/// 参数:
/// - `pid`: 进程 ID
/// - `pgid`: 进程组 ID
/// - `force`: 是否直接强杀
pub(crate) async fn terminate_process(pid: u32, pgid: Option<i32>, force: bool) {
    #[cfg(unix)]
    {
        let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
        let target = pgid.map(|pgid| -pgid).unwrap_or(pid as i32);
        unsafe {
            libc::kill(target, signal);
        }
    }
    #[cfg(windows)]
    {
        let mut command = tokio::process::Command::new("taskkill");
        command.arg("/PID").arg(pid.to_string()).arg("/T");
        if force {
            command.arg("/F");
        }
        let _ = command.output().await;
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        let _ = pgid;
        let _ = force;
    }
}
