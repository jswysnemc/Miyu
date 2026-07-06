use super::model::{SessionInfo, DEFAULT_SESSION_ID};
use crate::paths::MiyuPaths;
use anyhow::{bail, Context, Result};
use chrono::Utc;
use std::path::{Path, PathBuf};

/// 读取会话列表。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - 会话列表
pub fn list_sessions(paths: &MiyuPaths) -> Result<Vec<SessionInfo>> {
    ensure_default_session(paths)
}

/// 创建新会话并设为当前会话。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `title`: 可选会话标题
///
/// 返回:
/// - 新会话信息
pub fn create_session(paths: &MiyuPaths, title: Option<&str>) -> Result<SessionInfo> {
    let now = Utc::now().to_rfc3339();
    let session = SessionInfo {
        id: new_session_id(),
        title: title
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| "New session".to_string()),
        created_at: now.clone(),
        updated_at: now,
    };
    let mut sessions = ensure_default_session(paths)?;
    sessions.insert(0, session.clone());
    save_sessions(paths, &sessions)?;
    write_current_session_id(paths, &session.id)?;
    std::fs::create_dir_all(session_state_dir(paths, &session.id))?;
    Ok(session)
}

/// 切换当前会话。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `session_id`: 会话 ID
///
/// 返回:
/// - 当前会话信息
pub fn switch_session(paths: &MiyuPaths, session_id: &str) -> Result<SessionInfo> {
    let session_id = session_id.trim();
    let session = ensure_default_session(paths)?
        .into_iter()
        .find(|session| session.id == session_id)
        .with_context(|| format!("session not found: {session_id}"))?;
    write_current_session_id(paths, &session.id)?;
    std::fs::create_dir_all(session_state_dir(paths, &session.id))?;
    Ok(session)
}

/// 重命名会话。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `session_id`: 会话 ID
/// - `title`: 新标题
///
/// 返回:
/// - 更新后的会话信息
pub fn rename_session(paths: &MiyuPaths, session_id: &str, title: &str) -> Result<SessionInfo> {
    let title = title.trim();
    if title.is_empty() {
        bail!("session title cannot be empty");
    }
    let mut sessions = ensure_default_session(paths)?;
    let session = sessions
        .iter_mut()
        .find(|session| session.id == session_id.trim())
        .with_context(|| format!("session not found: {}", session_id.trim()))?;
    session.title = title.to_string();
    session.updated_at = Utc::now().to_rfc3339();
    let updated = session.clone();
    sort_sessions(&mut sessions);
    save_sessions(paths, &sessions)?;
    Ok(updated)
}

/// 删除会话。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `session_id`: 会话 ID
///
/// 返回:
/// - 是否删除了会话
pub fn delete_session(paths: &MiyuPaths, session_id: &str) -> Result<bool> {
    let session_id = session_id.trim();
    if session_id == DEFAULT_SESSION_ID {
        bail!("default session cannot be deleted");
    }
    let mut sessions = ensure_default_session(paths)?;
    let before = sessions.len();
    sessions.retain(|session| session.id != session_id);
    if before == sessions.len() {
        return Ok(false);
    }
    save_sessions(paths, &sessions)?;
    let state_dir = session_state_dir(paths, session_id);
    if state_dir.exists() {
        std::fs::remove_dir_all(state_dir)?;
    }
    if read_current_session_id(paths)? == session_id {
        write_current_session_id(paths, DEFAULT_SESSION_ID)?;
    }
    Ok(true)
}

/// 确保当前会话存在。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - 当前会话信息
pub fn ensure_active_session(paths: &MiyuPaths) -> Result<SessionInfo> {
    let sessions = ensure_default_session(paths)?;
    let active_id = read_current_session_id(paths)?;
    if let Some(session) = sessions.iter().find(|session| session.id == active_id) {
        std::fs::create_dir_all(session_state_dir(paths, &session.id))?;
        return Ok(session.clone());
    }
    write_current_session_id(paths, DEFAULT_SESSION_ID)?;
    let session = sessions
        .into_iter()
        .find(|session| session.id == DEFAULT_SESSION_ID)
        .expect("default session must exist");
    Ok(session)
}

/// 返回当前会话状态目录。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - 当前会话状态目录
pub fn active_state_dir(paths: &MiyuPaths) -> Result<PathBuf> {
    let session = ensure_active_session(paths)?;
    Ok(session_state_dir(paths, &session.id))
}

/// 根据用户消息更新会话标题和更新时间。
///
/// 参数:
/// - `base_state_dir`: 原始状态目录
/// - `session_id`: 会话 ID
/// - `message`: 用户消息
///
/// 返回:
/// - 更新是否成功
pub fn touch_session_with_message(
    base_state_dir: &Path,
    session_id: &str,
    message: &str,
) -> Result<()> {
    let mut sessions = ensure_default_session_for_base(base_state_dir)?;
    let now = Utc::now().to_rfc3339();
    if let Some(session) = sessions.iter_mut().find(|session| session.id == session_id) {
        if session.title == "New session" || session.title == "Default" {
            session.title = title_from_message(message, &session.title);
        }
        session.updated_at = now;
    }
    sort_sessions(&mut sessions);
    save_sessions_to_base(base_state_dir, &sessions)
}

/// 确保默认会话存在。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - 会话列表
fn ensure_default_session(paths: &MiyuPaths) -> Result<Vec<SessionInfo>> {
    ensure_default_session_for_base(&paths.state_dir)
}

/// 确保默认会话存在。
///
/// 参数:
/// - `base_state_dir`: 原始状态目录
///
/// 返回:
/// - 会话列表
fn ensure_default_session_for_base(base_state_dir: &Path) -> Result<Vec<SessionInfo>> {
    std::fs::create_dir_all(base_state_dir)?;
    let mut sessions = read_sessions_from_base(base_state_dir)?;
    if !sessions
        .iter()
        .any(|session| session.id == DEFAULT_SESSION_ID)
    {
        let now = Utc::now().to_rfc3339();
        sessions.push(SessionInfo::default_with_time(&now));
    }
    sort_sessions(&mut sessions);
    save_sessions_to_base(base_state_dir, &sessions)?;
    Ok(sessions)
}

/// 读取当前会话 ID。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - 当前会话 ID
fn read_current_session_id(paths: &MiyuPaths) -> Result<String> {
    let file = current_session_file(&paths.state_dir);
    if !file.exists() {
        write_current_session_id(paths, DEFAULT_SESSION_ID)?;
        return Ok(DEFAULT_SESSION_ID.to_string());
    }
    let value = std::fs::read_to_string(file)?;
    Ok(value.trim().to_string())
}

/// 写入当前会话 ID。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `session_id`: 会话 ID
///
/// 返回:
/// - 写入是否成功
fn write_current_session_id(paths: &MiyuPaths, session_id: &str) -> Result<()> {
    if let Some(parent) = current_session_file(&paths.state_dir).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        current_session_file(&paths.state_dir),
        format!("{session_id}\n"),
    )?;
    Ok(())
}

/// 读取会话索引。
///
/// 参数:
/// - `base_state_dir`: 原始状态目录
///
/// 返回:
/// - 会话列表
fn read_sessions_from_base(base_state_dir: &Path) -> Result<Vec<SessionInfo>> {
    let file = sessions_file(base_state_dir);
    if !file.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&file)
        .with_context(|| format!("failed to read {}", file.display()))?;
    Ok(
        serde_json::from_str(&raw)
            .with_context(|| format!("invalid JSON in {}", file.display()))?,
    )
}

/// 保存会话索引。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `sessions`: 会话列表
///
/// 返回:
/// - 保存是否成功
fn save_sessions(paths: &MiyuPaths, sessions: &[SessionInfo]) -> Result<()> {
    save_sessions_to_base(&paths.state_dir, sessions)
}

/// 保存会话索引。
///
/// 参数:
/// - `base_state_dir`: 原始状态目录
/// - `sessions`: 会话列表
///
/// 返回:
/// - 保存是否成功
fn save_sessions_to_base(base_state_dir: &Path, sessions: &[SessionInfo]) -> Result<()> {
    let file = sessions_file(base_state_dir);
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
/// - `base_state_dir`: 原始状态目录
///
/// 返回:
/// - 会话索引文件路径
fn sessions_file(base_state_dir: &Path) -> PathBuf {
    base_state_dir.join("sessions").join("index.json")
}

/// 返回当前会话文件路径。
///
/// 参数:
/// - `base_state_dir`: 原始状态目录
///
/// 返回:
/// - 当前会话文件路径
fn current_session_file(base_state_dir: &Path) -> PathBuf {
    base_state_dir.join("sessions").join("current")
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
    if session_id == DEFAULT_SESSION_ID {
        return paths.state_dir.clone();
    }
    paths
        .state_dir
        .join("sessions")
        .join("data")
        .join(sanitize_session_id(session_id))
}

/// 生成新会话 ID。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 会话 ID
fn new_session_id() -> String {
    format!(
        "session_{}_{}",
        Utc::now().timestamp_millis(),
        rand::random::<u16>()
    )
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
        DEFAULT_SESSION_ID.to_string()
    } else {
        value
    }
}

/// 从用户消息生成会话标题。
///
/// 参数:
/// - `message`: 用户消息
/// - `fallback`: 默认标题
///
/// 返回:
/// - 会话标题
fn title_from_message(message: &str, fallback: &str) -> String {
    let title = message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(32)
        .collect::<String>();
    if title.trim().is_empty() {
        fallback.to_string()
    } else {
        title
    }
}

/// 按更新时间排序会话。
///
/// 参数:
/// - `sessions`: 会话列表
///
/// 返回:
/// - 无
fn sort_sessions(sessions: &mut [SessionInfo]) {
    sessions.sort_by(|a, b| {
        if a.id == DEFAULT_SESSION_ID {
            std::cmp::Ordering::Greater
        } else if b.id == DEFAULT_SESSION_ID {
            std::cmp::Ordering::Less
        } else {
            b.updated_at.cmp(&a.updated_at)
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_paths(root: PathBuf) -> MiyuPaths {
        MiyuPaths {
            config_dir: root.join("config"),
            config_file: root.join("config/config.jsonc"),
            secrets_file: root.join("config/secrets.jsonc"),
            skills_dir: root.join("config/skills"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            state_dir: root.join("state"),
            pictures_dir: root.join("pictures"),
            fish_hook_file: root.join("fish/miyu.fish"),
            bash_hook_file: root.join("shell/bash-hook.sh"),
            zsh_hook_file: root.join("shell/zsh-hook.zsh"),
            powershell_hook_file: root.join("shell/powershell-hook.ps1"),
        }
    }

    #[test]
    fn default_session_uses_root_state_dir() {
        let temp = tempfile::tempdir().unwrap();
        let paths = test_paths(temp.path().to_path_buf());

        let session = ensure_active_session(&paths).unwrap();

        assert_eq!(session.id, DEFAULT_SESSION_ID);
        assert_eq!(active_state_dir(&paths).unwrap(), paths.state_dir);
    }

    #[test]
    fn create_session_switches_current_session() {
        let temp = tempfile::tempdir().unwrap();
        let paths = test_paths(temp.path().to_path_buf());

        let session = create_session(&paths, Some("Work")).unwrap();
        let active = ensure_active_session(&paths).unwrap();

        assert_eq!(active.id, session.id);
        assert!(active_state_dir(&paths).unwrap().ends_with(&session.id));
    }

    #[test]
    fn delete_active_session_switches_to_default() {
        let temp = tempfile::tempdir().unwrap();
        let paths = test_paths(temp.path().to_path_buf());
        let session = create_session(&paths, Some("Work")).unwrap();

        assert!(delete_session(&paths, &session.id).unwrap());

        assert_eq!(
            ensure_active_session(&paths).unwrap().id,
            DEFAULT_SESSION_ID
        );
    }

    #[test]
    fn touch_updates_new_session_title() {
        let temp = tempfile::tempdir().unwrap();
        let paths = test_paths(temp.path().to_path_buf());
        let session = create_session(&paths, None).unwrap();

        touch_session_with_message(&paths.state_dir, &session.id, "hello project world").unwrap();
        let updated = list_sessions(&paths)
            .unwrap()
            .into_iter()
            .find(|item| item.id == session.id)
            .unwrap();

        assert_eq!(updated.title, "hello project world");
    }
}
