use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use std::time::Duration;

pub(crate) fn read_key() -> Result<KeyCode> {
    Ok(read_key_event()?.code)
}

pub(crate) fn read_key_with_timeout(timeout: Option<Duration>) -> Result<Option<KeyCode>> {
    read_key_event_with_timeout(timeout).map(|key| key.map(|key| key.code))
}

pub(crate) fn read_key_event() -> Result<KeyEvent> {
    read_key_event_with_timeout(None).map(|key| key.expect("blocking read should return a key"))
}

pub(crate) fn read_key_event_with_timeout(timeout: Option<Duration>) -> Result<Option<KeyEvent>> {
    loop {
        if let Some(timeout) = timeout {
            if !event::poll(timeout)? {
                return Ok(None);
            }
        }
        if let Event::Key(event) = event::read()? {
            if !is_actionable_key_event(event) {
                continue;
            }
            return Ok(Some(event));
        }
    }
}

/// 判断键盘事件是否可以驱动配置界面操作。
///
/// 参数:
/// - `event`: 终端键盘事件
///
/// 返回:
/// - 是否属于按下或重复输入事件
fn is_actionable_key_event(event: KeyEvent) -> bool {
    event.kind != KeyEventKind::Release
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};

    /// 验证配置界面忽略 Windows 控制台残留的按键释放事件。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn ignores_key_release_events() {
        let release = KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        };

        assert!(!is_actionable_key_event(release));
    }
}
