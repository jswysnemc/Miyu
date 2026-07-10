use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Debug)]
pub(crate) struct ProcessUsageSnapshot {
    pub pid: u32,
    pub uptime_seconds: u64,
    pub rss_bytes: Option<u64>,
    pub cpu_percent: f64,
}

#[derive(Clone)]
pub(crate) struct SystemMonitor {
    started_at: Instant,
    previous: Arc<Mutex<CpuSample>>,
}

#[derive(Clone, Copy)]
struct CpuSample {
    at: Instant,
    cpu_seconds: f64,
}

impl SystemMonitor {
    /// 创建进程用量采样器。
    ///
    /// 返回:
    /// - 系统用量采样器
    pub(crate) fn new() -> Self {
        let now = Instant::now();
        Self {
            started_at: now,
            previous: Arc::new(Mutex::new(CpuSample {
                at: now,
                cpu_seconds: process_cpu_seconds(),
            })),
        }
    }

    /// 读取当前进程用量并更新 CPU 采样基线。
    ///
    /// 返回:
    /// - 进程用量快照
    pub(crate) fn snapshot(&self) -> ProcessUsageSnapshot {
        let now = Instant::now();
        let cpu_seconds = process_cpu_seconds();
        let cpu_percent = self
            .previous
            .lock()
            .map(|mut previous| {
                let wall_seconds = now.duration_since(previous.at).as_secs_f64();
                let cpu_delta = (cpu_seconds - previous.cpu_seconds).max(0.0);
                *previous = CpuSample {
                    at: now,
                    cpu_seconds,
                };
                if wall_seconds > 0.0 {
                    cpu_delta / wall_seconds * 100.0
                } else {
                    0.0
                }
            })
            .unwrap_or_default();
        ProcessUsageSnapshot {
            pid: std::process::id(),
            uptime_seconds: now.duration_since(self.started_at).as_secs(),
            rss_bytes: process_rss_bytes(),
            cpu_percent,
        }
    }
}

/// 读取当前进程累计 CPU 秒数。
///
/// 返回:
/// - 用户态和内核态 CPU 秒数之和
fn process_cpu_seconds() -> f64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status != 0 {
        return 0.0;
    }
    let usage = unsafe { usage.assume_init() };
    timeval_seconds(usage.ru_utime) + timeval_seconds(usage.ru_stime)
}

/// 将 libc timeval 转换为秒。
///
/// 参数:
/// - `value`: timeval 值
///
/// 返回:
/// - 浮点秒数
fn timeval_seconds(value: libc::timeval) -> f64 {
    value.tv_sec as f64 + value.tv_usec as f64 / 1_000_000.0
}

/// 读取 Linux 当前进程常驻内存。
///
/// 返回:
/// - 常驻内存字节数，当前平台不可用时返回空值
#[cfg(target_os = "linux")]
fn process_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    let kib = line.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    Some(kib * 1024)
}

/// 非 Linux 平台暂不提供常驻内存。
///
/// 返回:
/// - 空值
#[cfg(not(target_os = "linux"))]
fn process_rss_bytes() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_current_process_metrics() {
        let monitor = SystemMonitor::new();
        let snapshot = monitor.snapshot();
        assert_eq!(snapshot.pid, std::process::id());
        assert!(snapshot.cpu_percent >= 0.0);
        #[cfg(target_os = "linux")]
        assert!(snapshot.rss_bytes.unwrap_or_default() > 0);
    }
}
