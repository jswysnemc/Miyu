use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::time::Duration;

pub(crate) fn read_key() -> Result<KeyCode> {
    read_key_with_timeout(None).map(|key| key.expect("blocking read should return a key"))
}

pub(crate) fn read_key_with_timeout(timeout: Option<Duration>) -> Result<Option<KeyCode>> {
    loop {
        if let Some(timeout) = timeout {
            if !event::poll(timeout)? {
                return Ok(None);
            }
        }
        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            return Ok(Some(code));
        }
    }
}
