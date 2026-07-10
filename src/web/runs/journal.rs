use super::WebEvent;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

const EVENT_JOURNAL_CAPACITY: usize = 2048;
const EVENT_BROADCAST_CAPACITY: usize = 512;

/// 单轮运行的有界事件日志与实时广播。
#[derive(Clone)]
pub(crate) struct EventJournal {
    inner: Arc<Mutex<JournalInner>>,
    sender: broadcast::Sender<WebEvent>,
}

struct JournalInner {
    next_sequence: u64,
    events: VecDeque<WebEvent>,
}

impl EventJournal {
    /// 创建空事件日志。
    ///
    /// 返回:
    /// - 事件日志
    pub(crate) fn new() -> Self {
        let (sender, _) = broadcast::channel(EVENT_BROADCAST_CAPACITY);
        Self {
            inner: Arc::new(Mutex::new(JournalInner {
                next_sequence: 1,
                events: VecDeque::new(),
            })),
            sender,
        }
    }

    /// 写入并广播事件。
    ///
    /// 参数:
    /// - `event`: 尚未分配序号的事件
    ///
    /// 返回:
    /// - 已分配序号的事件
    pub(crate) fn publish(&self, mut event: WebEvent) -> WebEvent {
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        event.sequence = inner.next_sequence;
        inner.next_sequence = inner.next_sequence.saturating_add(1);
        inner.events.push_back(event.clone());
        while inner.events.len() > EVENT_JOURNAL_CAPACITY {
            inner.events.pop_front();
        }
        drop(inner);
        let _ = self.sender.send(event.clone());
        event
    }

    /// 返回指定序号之后的历史事件。
    ///
    /// 参数:
    /// - `after`: 已接收的最后事件序号
    ///
    /// 返回:
    /// - 需要补发的事件
    pub(crate) fn events_after(&self, after: u64) -> Vec<WebEvent> {
        let inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        inner
            .events
            .iter()
            .filter(|event| event.sequence > after)
            .cloned()
            .collect()
    }

    /// 订阅实时事件。
    ///
    /// 返回:
    /// - 广播接收器
    pub(crate) fn subscribe(&self) -> broadcast::Receiver<WebEvent> {
        self.sender.subscribe()
    }
}
