use super::model::{SessionInfo, DEFAULT_SESSION_ID};
use super::workspace::{current_workspace_scope, workspace_scope_for_path, WorkspaceScope};
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
    let scope = current_session_scope(paths)?;
    ensure_default_session_for_base(&scope.state_dir)
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
    let scope = current_session_scope(paths)?;
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
    let mut sessions = ensure_default_session_for_base(&scope.state_dir)?;
    sessions.insert(0, session.clone());
    save_sessions_to_base(&scope.state_dir, &sessions)?;
    write_current_session_id_to_base(&scope.state_dir, &session.id)?;
    std::fs::create_dir_all(session_state_dir(&scope.state_dir, &session.id))?;
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
    let scope = current_session_scope(paths)?;
    let session_id = session_id.trim();
    let session = ensure_default_session_for_base(&scope.state_dir)?
        .into_iter()
        .find(|session| session.id == session_id)
        .with_context(|| format!("session not found: {session_id}"))?;
    write_current_session_id_to_base(&scope.state_dir, &session.id)?;
    std::fs::create_dir_all(session_state_dir(&scope.state_dir, &session.id))?;
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
    let scope = current_session_scope(paths)?;
    let title = title.trim();
    if title.is_empty() {
        bail!("session title cannot be empty");
    }
    let mut sessions = ensure_default_session_for_base(&scope.state_dir)?;
    let session = sessions
        .iter_mut()
        .find(|session| session.id == session_id.trim())
        .with_context(|| format!("session not found: {}", session_id.trim()))?;
    session.title = title.to_string();
    session.updated_at = Utc::now().to_rfc3339();
    let updated = session.clone();
    sort_sessions(&mut sessions);
    save_sessions_to_base(&scope.state_dir, &sessions)?;
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
    Ok(!delete_sessions(paths, &[session_id.to_string()])?.is_empty())
}

/// 批量删除会话并仅写入一次索引。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `session_ids`: 待删除会话 ID 列表
///
/// 返回:
/// - 实际删除的会话 ID 列表
pub fn delete_sessions(paths: &MiyuPaths, session_ids: &[String]) -> Result<Vec<String>> {
    let scope = current_session_scope(paths)?;
    let requested = session_ids
        .iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
        .collect::<std::collections::BTreeSet<_>>();
    if requested.contains(DEFAULT_SESSION_ID) {
        bail!("default session cannot be deleted");
    }
    let mut sessions = ensure_default_session_for_base(&scope.state_dir)?;
    let deleted = sessions
        .iter()
        .filter(|session| requested.contains(session.id.as_str()))
        .map(|session| session.id.clone())
        .collect::<Vec<_>>();
    if deleted.is_empty() {
        return Ok(Vec::new());
    }
    sessions.retain(|session| !requested.contains(session.id.as_str()));
    save_sessions_to_base(&scope.state_dir, &sessions)?;
    for session_id in &deleted {
        let state_dir = session_state_dir(&scope.state_dir, session_id);
        if state_dir.exists() {
            std::fs::remove_dir_all(state_dir)?;
        }
    }
    if deleted.contains(&read_current_session_id_from_base(&scope.state_dir)?) {
        write_current_session_id_to_base(&scope.state_dir, DEFAULT_SESSION_ID)?;
    }
    Ok(deleted)
}

/// 确保当前会话存在。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - 当前会话信息
pub fn ensure_active_session(paths: &MiyuPaths) -> Result<SessionInfo> {
    let scope = current_session_scope(paths)?;
    let sessions = ensure_default_session_for_base(&scope.state_dir)?;
    let active_id = read_current_session_id_from_base(&scope.state_dir)?;
    if let Some(session) = sessions.iter().find(|session| session.id == active_id) {
        std::fs::create_dir_all(session_state_dir(&scope.state_dir, &session.id))?;
        return Ok(session.clone());
    }
    write_current_session_id_to_base(&scope.state_dir, DEFAULT_SESSION_ID)?;
    let session = sessions
        .into_iter()
        .find(|session| session.id == DEFAULT_SESSION_ID)
        .expect("default session must exist");
    std::fs::create_dir_all(session_state_dir(&scope.state_dir, &session.id))?;
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
    let scope = current_session_scope(paths)?;
    let session = ensure_active_session(paths)?;
    Ok(session_state_dir(&scope.state_dir, &session.id))
}

/// 返回指定会话的状态目录，并校验会话存在。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `session_id`: 会话 ID
///
/// 返回:
/// - 指定会话状态目录
pub fn state_dir_for_session(paths: &MiyuPaths, session_id: &str) -> Result<PathBuf> {
    let scope = current_session_scope(paths)?;
    let session_id = session_id.trim();
    ensure_default_session_for_base(&scope.state_dir)?
        .into_iter()
        .find(|session| session.id == session_id)
        .with_context(|| format!("session not found: {session_id}"))?;
    let state_dir = session_state_dir(&scope.state_dir, session_id);
    std::fs::create_dir_all(&state_dir)?;
    Ok(state_dir)
}

/// 返回指定工作区和会话的状态目录。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `workspace_path`: 工作区目录
/// - `session_id`: 会话 ID
///
/// 返回:
/// - 指定会话状态目录
pub fn state_dir_for_workspace_session(
    paths: &MiyuPaths,
    workspace_path: &Path,
    session_id: &str,
) -> Result<(PathBuf, PathBuf)> {
    let scope = workspace_scope_for_path(paths, workspace_path);
    migrate_legacy_sessions_to_workspace(paths, &scope.state_dir)?;
    let session_id = session_id.trim();
    ensure_default_session_for_base(&scope.state_dir)?
        .into_iter()
        .find(|session| session.id == session_id)
        .with_context(|| format!("session not found: {session_id}"))?;
    let state_dir = session_state_dir(&scope.state_dir, session_id);
    std::fs::create_dir_all(&state_dir)?;
    Ok((scope.state_dir, state_dir))
}

/// 返回当前工作区会话作用域目录。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - 当前工作区会话作用域目录
pub fn session_scope_dir(paths: &MiyuPaths) -> Result<PathBuf> {
    Ok(current_session_scope(paths)?.state_dir)
}

/// 返回完成旧状态迁移后的当前工作区会话作用域。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - 当前工作区会话作用域
fn current_session_scope(paths: &MiyuPaths) -> Result<WorkspaceScope> {
    let scope = current_workspace_scope(paths)?;
    migrate_legacy_sessions_to_workspace(paths, &scope.state_dir)?;
    Ok(scope)
}

/// 首次使用工作区会话时迁移旧版全局会话状态。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `workspace_state_dir`: 当前工作区会话作用域目录
///
/// 返回:
/// - 迁移是否成功
fn migrate_legacy_sessions_to_workspace(
    paths: &MiyuPaths,
    workspace_state_dir: &Path,
) -> Result<()> {
    if workspace_has_state(workspace_state_dir)? {
        return Ok(());
    }

    let legacy_sessions_dir = paths.state_dir.join("sessions");
    let legacy_index = legacy_sessions_dir.join("index.json");
    let legacy_current = legacy_sessions_dir.join("current");
    let legacy_data_dir = legacy_sessions_dir.join("data");
    let has_legacy_sessions =
        legacy_index.exists() || legacy_current.exists() || legacy_data_dir.exists();
    let has_legacy_default = legacy_default_files()
        .iter()
        .any(|name| paths.state_dir.join(name).exists());
    if !has_legacy_sessions && !has_legacy_default {
        return Ok(());
    }

    std::fs::create_dir_all(workspace_state_dir)?;
    copy_file_if_missing(&legacy_index, &sessions_file(workspace_state_dir))?;
    copy_file_if_missing(&legacy_current, &current_session_file(workspace_state_dir))?;
    copy_dir_contents_if_missing(&legacy_data_dir, &workspace_state_dir.join("data"))?;

    if has_legacy_default {
        let default_dir = session_state_dir(workspace_state_dir, DEFAULT_SESSION_ID);
        std::fs::create_dir_all(&default_dir)?;
        for name in legacy_default_files() {
            copy_file_if_missing(&paths.state_dir.join(name), &default_dir.join(name))?;
        }
    }
    Ok(())
}

/// 判断工作区会话作用域是否已经存在有效状态。
///
/// 参数:
/// - `workspace_state_dir`: 当前工作区会话作用域目录
///
/// 返回:
/// - 存在状态时返回 true
fn workspace_has_state(workspace_state_dir: &Path) -> Result<bool> {
    if sessions_file(workspace_state_dir).exists()
        || current_session_file(workspace_state_dir).exists()
    {
        return Ok(true);
    }
    has_dir_entries(&workspace_state_dir.join("data"))
}

/// 返回旧版默认会话状态文件名。
///
/// 返回:
/// - 文件名列表
fn legacy_default_files() -> &'static [&'static str] {
    &[
        "conversation.db",
        "conversation.jsonl",
        "usage.json",
        "loaded-tools.json",
        "miyu.log",
        "profile.md",
        "compaction-summary.json",
        "prompt.sha256",
    ]
}

/// 判断目录是否存在条目。
///
/// 参数:
/// - `path`: 目录路径
///
/// 返回:
/// - 存在条目时返回 true
fn has_dir_entries(path: &Path) -> Result<bool> {
    if !path.is_dir() {
        return Ok(false);
    }
    Ok(std::fs::read_dir(path)?.next().transpose()?.is_some())
}

/// 复制文件，目标已存在时跳过。
///
/// 参数:
/// - `source`: 源文件路径
/// - `target`: 目标文件路径
///
/// 返回:
/// - 复制是否成功
fn copy_file_if_missing(source: &Path, target: &Path) -> Result<()> {
    if !source.is_file() || target.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(source, target)?;
    Ok(())
}

/// 递归复制目录内容，目标已存在的条目会跳过。
///
/// 参数:
/// - `source`: 源目录路径
/// - `target`: 目标目录路径
///
/// 返回:
/// - 复制是否成功
fn copy_dir_contents_if_missing(source: &Path, target: &Path) -> Result<()> {
    if !source.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if target_path.exists() {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir_contents_if_missing(&source_path, &target_path)?;
        } else if file_type.is_file() {
            copy_file_if_missing(&source_path, &target_path)?;
        }
    }
    Ok(())
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
fn read_current_session_id_from_base(base_state_dir: &Path) -> Result<String> {
    let file = current_session_file(base_state_dir);
    if !file.exists() {
        write_current_session_id_to_base(base_state_dir, DEFAULT_SESSION_ID)?;
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
fn write_current_session_id_to_base(base_state_dir: &Path, session_id: &str) -> Result<()> {
    if let Some(parent) = current_session_file(base_state_dir).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        current_session_file(base_state_dir),
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
    base_state_dir.join("index.json")
}

/// 返回当前会话文件路径。
///
/// 参数:
/// - `base_state_dir`: 原始状态目录
///
/// 返回:
/// - 当前会话文件路径
fn current_session_file(base_state_dir: &Path) -> PathBuf {
    base_state_dir.join("current")
}

/// 返回会话状态目录。
///
/// 参数:
/// - `base_state_dir`: 当前工作区会话作用域目录
/// - `session_id`: 会话 ID
///
/// 返回:
/// - 会话状态目录
fn session_state_dir(base_state_dir: &Path, session_id: &str) -> PathBuf {
    base_state_dir
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
    fn default_session_uses_workspace_state_dir() {
        let temp = tempfile::tempdir().unwrap();
        let paths = test_paths(temp.path().to_path_buf());

        let session = ensure_active_session(&paths).unwrap();
        let scope_dir = session_scope_dir(&paths).unwrap();

        assert_eq!(session.id, DEFAULT_SESSION_ID);
        assert_eq!(
            active_state_dir(&paths).unwrap(),
            scope_dir.join("data").join(DEFAULT_SESSION_ID)
        );
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
    fn deletes_multiple_sessions_with_one_index_update() {
        let temp = tempfile::tempdir().unwrap();
        let paths = test_paths(temp.path().to_path_buf());
        let first = create_session(&paths, Some("First")).unwrap();
        let second = create_session(&paths, Some("Second")).unwrap();

        let deleted = delete_sessions(&paths, &[first.id.clone(), second.id.clone()]).unwrap();
        let remaining = list_sessions(&paths).unwrap();

        assert_eq!(deleted.len(), 2);
        assert!(remaining
            .iter()
            .all(|session| session.id == DEFAULT_SESSION_ID));
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

        let scope_dir = session_scope_dir(&paths).unwrap();
        touch_session_with_message(&scope_dir, &session.id, "hello project world").unwrap();
        let updated = list_sessions(&paths)
            .unwrap()
            .into_iter()
            .find(|item| item.id == session.id)
            .unwrap();

        assert_eq!(updated.title, "hello project world");
    }

    #[test]
    fn migrates_legacy_sessions_into_workspace_scope() {
        let temp = tempfile::tempdir().unwrap();
        let paths = test_paths(temp.path().to_path_buf());
        let legacy_session = SessionInfo {
            id: "session_old".to_string(),
            title: "Old work".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let legacy_sessions_dir = paths.state_dir.join("sessions");
        std::fs::create_dir_all(legacy_sessions_dir.join("data/session_old")).unwrap();
        std::fs::create_dir_all(&paths.state_dir).unwrap();
        std::fs::write(
            legacy_sessions_dir.join("index.json"),
            serde_json::to_string_pretty(&vec![legacy_session.clone()]).unwrap(),
        )
        .unwrap();
        std::fs::write(legacy_sessions_dir.join("current"), "session_old\n").unwrap();
        std::fs::write(
            legacy_sessions_dir.join("data/session_old/usage.json"),
            "old session usage",
        )
        .unwrap();
        std::fs::write(paths.state_dir.join("conversation.jsonl"), "legacy default").unwrap();

        let active = ensure_active_session(&paths).unwrap();
        let scope_dir = session_scope_dir(&paths).unwrap();

        assert_eq!(active.id, "session_old");
        assert!(sessions_file(&scope_dir).exists());
        assert_eq!(
            std::fs::read_to_string(current_session_file(&scope_dir)).unwrap(),
            "session_old\n"
        );
        assert_eq!(
            std::fs::read_to_string(scope_dir.join("data/session_old/usage.json")).unwrap(),
            "old session usage"
        );
        assert_eq!(
            std::fs::read_to_string(
                session_state_dir(&scope_dir, DEFAULT_SESSION_ID).join("conversation.jsonl")
            )
            .unwrap(),
            "legacy default"
        );
    }
}
