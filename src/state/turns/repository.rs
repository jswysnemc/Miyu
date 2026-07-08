use super::model::{interrupted_text, pending_placeholder, Turn, TurnStatus};
use super::schema::open_connection;
use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::path::Path;
use std::sync::Mutex;

pub struct ConversationDb {
    pub(in crate::state) conn: Mutex<Connection>,
}

impl std::fmt::Debug for ConversationDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConversationDb").finish_non_exhaustive()
    }
}

impl ConversationDb {
    /// 打开对话数据库。
    ///
    /// 参数:
    /// - `state_dir`: 状态目录
    ///
    /// 返回:
    /// - 对话数据库仓储
    pub fn open(state_dir: &Path) -> Result<Self> {
        Ok(Self {
            conn: Mutex::new(open_connection(state_dir)?),
        })
    }

    /// 使用数据库连接执行只暴露连接边界的操作。
    ///
    /// 参数:
    /// - `operation`: 需要在连接上执行的操作
    ///
    /// 返回:
    /// - 操作结果
    pub(crate) fn with_conn<T>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T>,
    ) -> Result<T> {
        let conn = self.conn.lock().unwrap();
        operation(&conn)
    }

    /// 开始一轮新对话。
    ///
    /// 参数:
    /// - `turn_id`: 当前轮唯一标识
    /// - `user_content`: 用户输入
    ///
    /// 返回:
    /// - 写入是否成功
    pub fn start_turn(&self, turn_id: &str, user_content: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        self.insert_turn_locked(
            &conn,
            InsertTurn {
                turn_id,
                user_content,
                user_timestamp: &now,
                assistant_content: pending_placeholder(),
                assistant_reasoning: None,
                assistant_timestamp: None,
                status: TurnStatus::Running,
                tool_reports: &[],
            },
        )
    }

    /// 完成指定对话轮次。
    ///
    /// 参数:
    /// - `turn_id`: 当前轮唯一标识
    /// - `content`: 助手回复
    /// - `reasoning`: 可选推理内容
    ///
    /// 返回:
    /// - 更新是否成功
    pub fn complete_turn(
        &self,
        turn_id: &str,
        content: &str,
        reasoning: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE turns
             SET assistant_content = ?1,
                 assistant_reasoning = ?2,
                 assistant_timestamp = ?3,
                 status = 'completed'
             WHERE turn_id = ?4",
            params![content, reasoning, now, turn_id],
        )?;
        Ok(())
    }

    /// 标记指定轮次已中断。
    ///
    /// 参数:
    /// - `turn_id`: 当前轮唯一标识
    ///
    /// 返回:
    /// - 更新是否成功
    pub fn interrupt_turn(&self, turn_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE turns
             SET assistant_content = ?1,
                 assistant_timestamp = ?2,
                 status = 'interrupted'
             WHERE turn_id = ?3 AND status = 'running'",
            params![interrupted_text(), now, turn_id],
        )?;
        Ok(())
    }

    /// 附加当前轮工具报告上下文。
    ///
    /// 参数:
    /// - `turn_id`: 当前轮唯一标识
    /// - `report`: 工具报告文本
    ///
    /// 返回:
    /// - 写入是否成功
    pub fn append_tool_report(&self, turn_id: &str, report: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let existing: Option<String> = conn
            .query_row(
                "SELECT tool_reports FROM turns WHERE turn_id = ?1",
                params![turn_id],
                |row| row.get(0),
            )
            .optional()?;
        let mut reports: Vec<String> = existing
            .as_deref()
            .and_then(|value| serde_json::from_str(value).ok())
            .unwrap_or_default();
        reports.push(report.to_string());
        conn.execute(
            "UPDATE turns SET tool_reports = ?1 WHERE turn_id = ?2",
            params![serde_json::to_string(&reports)?, turn_id],
        )?;
        Ok(())
    }

    /// 读取全部轮次。
    ///
    /// 返回:
    /// - 按序排列的轮次列表
    pub fn load_turns(&self) -> Result<Vec<Turn>> {
        let conn = self.conn.lock().unwrap();
        load_turns_with_sql(
            &conn,
            "SELECT turn_id, seq, user_content, user_timestamp, assistant_content,
                    assistant_reasoning, assistant_timestamp, status, tool_reports
             FROM turns ORDER BY seq ASC",
            [],
        )
    }

    /// 读取指定 seq 之后的轮次。
    ///
    /// 参数:
    /// - `after_seq`: 起始 seq，不包含该 seq
    /// - `exclude_turn_id`: 可选排除轮次
    ///
    /// 返回:
    /// - tail turns
    pub(in crate::state) fn load_turns_after_seq(
        &self,
        after_seq: i64,
        exclude_turn_id: Option<&str>,
    ) -> Result<Vec<Turn>> {
        let conn = self.conn.lock().unwrap();
        match exclude_turn_id {
            Some(turn_id) => {
                let mut stmt = conn.prepare(
                    "SELECT turn_id, seq, user_content, user_timestamp, assistant_content,
                            assistant_reasoning, assistant_timestamp, status, tool_reports
                     FROM turns WHERE seq > ?1 AND turn_id != ?2 ORDER BY seq ASC",
                )?;
                let turns = stmt
                    .query_map(params![after_seq, turn_id], map_turn)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(turns)
            }
            None => load_turns_with_sql(
                &conn,
                "SELECT turn_id, seq, user_content, user_timestamp, assistant_content,
                        assistant_reasoning, assistant_timestamp, status, tool_reports
                 FROM turns WHERE seq > ?1 ORDER BY seq ASC",
                params![after_seq],
            ),
        }
    }

    /// 清空对话轮次。
    ///
    /// 返回:
    /// - 清空是否成功
    pub fn reset(&self) -> Result<()> {
        self.conn.lock().unwrap().execute_batch(
            "DELETE FROM turns;
             DELETE FROM compaction_checkpoints;
             DELETE FROM context_epoch_events;
             DELETE FROM context_epochs;
             DELETE FROM failure_recovery_records;
             DELETE FROM session_memory;
             DELETE FROM tool_calls;
             DELETE FROM tool_results;
             DELETE FROM tool_output_replacements;",
        )?;
        Ok(())
    }

    /// 撤销最后一轮对话。
    ///
    /// 返回:
    /// - 删除轮次数量和被撤销的用户输入
    pub fn undo_last_turn(&self) -> Result<(usize, Option<String>)> {
        let conn = self.conn.lock().unwrap();
        let last: Option<(String, String)> = conn
            .query_row(
                "SELECT turn_id, user_content FROM turns ORDER BY seq DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        match last {
            Some((turn_id, user_content)) => {
                conn.execute("DELETE FROM turns WHERE turn_id = ?1", params![turn_id])?;
                Ok((1, Some(user_content)))
            }
            None => Ok((0, None)),
        }
    }

    /// 恢复所有陈旧运行中轮次为中断状态。
    ///
    /// 返回:
    /// - 被恢复的轮次标识列表
    pub fn recover_stale_running_turns(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let stale_turn_ids = {
            let mut stmt = conn.prepare("SELECT turn_id FROM turns WHERE status = 'running'")?;
            let rows = stmt.query_map([], |row| row.get(0))?;
            rows.collect::<std::result::Result<Vec<String>, _>>()?
        };
        if stale_turn_ids.is_empty() {
            return Ok(Vec::new());
        }
        conn.execute(
            "UPDATE turns
             SET assistant_content = ?1,
                 assistant_timestamp = ?2,
                 status = 'interrupted'
             WHERE status = 'running'",
            params![interrupted_text(), now],
        )?;
        Ok(stale_turn_ids)
    }

    /// 是否存在运行中轮次。
    ///
    /// 返回:
    /// - 是否存在运行中轮次
    #[allow(dead_code)]
    pub fn has_running_turns(&self) -> Result<bool> {
        let count: i64 = self.conn.lock().unwrap().query_row(
            "SELECT COUNT(*) FROM turns WHERE status = 'running'",
            [],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub(super) fn next_seq_locked(&self, conn: &Connection) -> Result<i64> {
        let max_seq: i64 =
            conn.query_row("SELECT COALESCE(MAX(seq), 0) FROM turns", [], |row| {
                row.get(0)
            })?;
        Ok(max_seq + 1)
    }

    pub(super) fn insert_turn_locked(&self, conn: &Connection, turn: InsertTurn<'_>) -> Result<()> {
        conn.execute(
            "INSERT INTO turns (
                turn_id, seq, user_content, user_timestamp, assistant_content,
                assistant_reasoning, assistant_timestamp, status, tool_reports
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                turn.turn_id,
                self.next_seq_locked(conn)?,
                turn.user_content,
                turn.user_timestamp,
                turn.assistant_content,
                turn.assistant_reasoning,
                turn.assistant_timestamp,
                turn.status.as_str(),
                serde_json::to_string(turn.tool_reports)?,
            ],
        )?;
        Ok(())
    }
}

pub(super) struct InsertTurn<'a> {
    pub(super) turn_id: &'a str,
    pub(super) user_content: &'a str,
    pub(super) user_timestamp: &'a str,
    pub(super) assistant_content: &'a str,
    pub(super) assistant_reasoning: Option<&'a str>,
    pub(super) assistant_timestamp: Option<&'a str>,
    pub(super) status: TurnStatus,
    pub(super) tool_reports: &'a [String],
}

/// 用固定 SQL 加载轮次。
///
/// 参数:
/// - `conn`: 数据库连接
/// - `sql`: 查询语句
/// - `params`: 查询参数
///
/// 返回:
/// - 轮次列表
fn load_turns_with_sql<P>(conn: &Connection, sql: &str, params: P) -> Result<Vec<Turn>>
where
    P: rusqlite::Params,
{
    let mut stmt = conn.prepare(sql)?;
    let turns = stmt
        .query_map(params, map_turn)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(turns)
}

/// 从查询行恢复轮次。
///
/// 参数:
/// - `row`: 查询行
///
/// 返回:
/// - 轮次
fn map_turn(row: &Row<'_>) -> rusqlite::Result<Turn> {
    let tool_reports_json: String = row.get(8)?;
    let tool_reports = serde_json::from_str(&tool_reports_json).unwrap_or_default();
    let status: String = row.get(7)?;
    Ok(Turn {
        turn_id: row.get(0)?,
        seq: row.get(1)?,
        user_content: row.get(2)?,
        user_timestamp: row.get(3)?,
        assistant_content: row.get(4)?,
        assistant_reasoning: row.get(5)?,
        assistant_timestamp: row.get(6)?,
        status: TurnStatus::from_str(&status),
        tool_reports,
    })
}
