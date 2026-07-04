use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

/// 打开并初始化对话 SQLite 数据库。
///
/// 参数:
/// - `state_dir`: 状态目录
///
/// 返回:
/// - 已初始化的数据库连接
pub(super) fn open_connection(state_dir: &Path) -> Result<Connection> {
    std::fs::create_dir_all(state_dir)?;
    let db_path = state_dir.join("conversation.db");
    let conn = Connection::open(&db_path)
        .with_context(|| format!("failed to open conversation db: {}", db_path.display()))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA foreign_keys = ON;",
    )?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS turns (
            turn_id             TEXT PRIMARY KEY,
            seq                 INTEGER NOT NULL UNIQUE,
            user_content        TEXT NOT NULL,
            user_timestamp      TEXT NOT NULL,
            assistant_content   TEXT NOT NULL,
            assistant_reasoning TEXT,
            assistant_timestamp TEXT,
            status              TEXT NOT NULL DEFAULT 'running',
            tool_reports        TEXT NOT NULL DEFAULT '[]'
        );
        CREATE INDEX IF NOT EXISTS idx_turns_seq ON turns(seq);
        CREATE INDEX IF NOT EXISTS idx_turns_status ON turns(status);",
    )?;
    Ok(conn)
}
