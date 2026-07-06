use crate::llm::Usage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Default, Serialize, Deserialize)]
struct UsageState {
    requests: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_usage: Option<Usage>,
}

/// 累加模型用量并保存最近一次 provider usage。
///
/// 参数:
/// - `path`: 用量状态文件
/// - `usage`: 当前请求 provider 返回的用量
///
/// 返回:
/// - 保存是否成功
pub fn add_usage(path: &Path, usage: &Usage) -> Result<()> {
    let mut state = if path.exists() {
        let raw = std::fs::read_to_string(path)?;
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        UsageState::default()
    };
    state.requests += 1;
    state.prompt_tokens += usage.prompt_tokens;
    state.completion_tokens += usage.completion_tokens;
    state.total_tokens += usage.total_tokens;
    state.last_usage = Some(usage.clone());
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(&state)?))?;
    Ok(())
}

/// 读取最近一次 provider usage。
///
/// 参数:
/// - `path`: 用量状态文件
///
/// 返回:
/// - 最近一次 provider usage
pub fn last_usage(path: &Path) -> Result<Option<Usage>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path)?;
    let state = serde_json::from_str::<UsageState>(&raw).unwrap_or_default();
    Ok(state.last_usage)
}

/// 清空最近一次 provider usage。
///
/// 参数:
/// - `path`: 用量状态文件
///
/// 返回:
/// - 清空是否成功
pub fn clear_last_usage(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let raw = std::fs::read_to_string(path)?;
    let mut state = serde_json::from_str::<UsageState>(&raw).unwrap_or_default();
    state.last_usage = None;
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(&state)?))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_clears_last_usage() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("usage.json");
        let usage = Usage {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        };

        add_usage(&path, &usage).unwrap();
        assert_eq!(last_usage(&path).unwrap().unwrap().total_tokens, 15);

        clear_last_usage(&path).unwrap();
        assert!(last_usage(&path).unwrap().is_none());
    }
}
