use crate::paths::MiyuPaths;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WebSession {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

/// 读取 Web 会话列表。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - Web 会话列表
pub(crate) fn list_sessions(paths: &MiyuPaths) -> Result<Vec<WebSession>> {
    let file = sessions_file(paths);
    if !file.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(file)?;
    Ok(serde_json::from_str(&raw)?)
}

/// 创建 Web 会话。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - 新会话
pub(crate) fn create_session(paths: &MiyuPaths) -> Result<WebSession> {
    let now = Utc::now().to_rfc3339();
    let session = WebSession {
        id: format!(
            "{}-{}",
            Utc::now().timestamp_millis(),
            rand::random::<u16>()
        ),
        title: "新会话".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };
    let mut sessions = list_sessions(paths)?;
    sessions.insert(0, session.clone());
    save_sessions(paths, &sessions)?;
    std::fs::create_dir_all(session_state_dir(paths, &session.id))?;
    Ok(session)
}

/// 删除 Web 会话。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `session_id`: 会话 ID
///
/// 返回:
/// - 是否删除了会话记录
pub(crate) fn delete_session(paths: &MiyuPaths, session_id: &str) -> Result<bool> {
    let mut sessions = list_sessions(paths)?;
    let before = sessions.len();
    sessions.retain(|session| session.id != session_id);
    save_sessions(paths, &sessions)?;
    let dir = session_state_dir(paths, session_id);
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(before != sessions.len())
}

/// 确保会话存在。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `session_id`: 会话 ID
///
/// 返回:
/// - 会话信息
pub(crate) fn ensure_session(paths: &MiyuPaths, session_id: &str) -> Result<WebSession> {
    if let Some(session) = list_sessions(paths)?
        .into_iter()
        .find(|session| session.id == session_id)
    {
        std::fs::create_dir_all(session_state_dir(paths, session_id))?;
        return Ok(session);
    }
    let now = Utc::now().to_rfc3339();
    let session = WebSession {
        id: session_id.to_string(),
        title: "新会话".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };
    let mut sessions = list_sessions(paths)?;
    sessions.insert(0, session.clone());
    save_sessions(paths, &sessions)?;
    std::fs::create_dir_all(session_state_dir(paths, session_id))?;
    Ok(session)
}

/// 根据用户消息更新会话标题和更新时间。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `session_id`: 会话 ID
/// - `message`: 用户消息
///
/// 返回:
/// - 无
pub(crate) fn touch_session_with_message(
    paths: &MiyuPaths,
    session_id: &str,
    message: &str,
) -> Result<()> {
    let mut sessions = list_sessions(paths)?;
    let now = Utc::now().to_rfc3339();
    if let Some(session) = sessions.iter_mut().find(|session| session.id == session_id) {
        if session.title == "新会话" {
            session.title = title_from_message(message);
        }
        session.updated_at = now;
    }
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    save_sessions(paths, &sessions)
}

/// 返回会话专属路径。
///
/// 参数:
/// - `paths`: 全局 Miyu 路径
/// - `session_id`: 会话 ID
///
/// 返回:
/// - 会话专属 Miyu 路径
pub(crate) fn session_paths(paths: &MiyuPaths, session_id: &str) -> MiyuPaths {
    let mut scoped = paths.clone();
    scoped.state_dir = session_state_dir(paths, session_id);
    scoped
}

/// 保存 Web 会话列表。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `sessions`: 会话列表
///
/// 返回:
/// - 保存是否成功
fn save_sessions(paths: &MiyuPaths, sessions: &[WebSession]) -> Result<()> {
    let file = sessions_file(paths);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        file,
        format!("{}\n", serde_json::to_string_pretty(sessions)?),
    )?;
    Ok(())
}

/// 返回会话索引文件路径。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - 会话索引文件路径
fn sessions_file(paths: &MiyuPaths) -> PathBuf {
    paths.state_dir.join("web").join("sessions.json")
}

/// 返回会话状态目录。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `session_id`: 会话 ID
///
/// 返回:
/// - 会话状态目录
fn session_state_dir(paths: &MiyuPaths, session_id: &str) -> PathBuf {
    paths
        .state_dir
        .join("web")
        .join("sessions")
        .join(sanitize_session_id(session_id))
}

/// 清理会话 ID。
///
/// 参数:
/// - `session_id`: 原始会话 ID
///
/// 返回:
/// - 安全会话 ID
fn sanitize_session_id(session_id: &str) -> String {
    let value = session_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        .collect::<String>();
    if value.is_empty() {
        "default".to_string()
    } else {
        value
    }
}

/// 从消息生成会话标题。
///
/// 参数:
/// - `message`: 用户消息
///
/// 返回:
/// - 会话标题
fn title_from_message(message: &str) -> String {
    let title = message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(32)
        .collect::<String>();
    if title.trim().is_empty() {
        "新会话".to_string()
    } else {
        title
    }
}
