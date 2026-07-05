use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
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
            return Ok(Some(event));
        }
    }
}
