use super::repository::load_tool_exchanges_for_turn;
use crate::llm::{ChatMessage, ToolCall, ToolCallFunction};
use crate::state::tool_history::project_legacy_tool_report_messages;
use crate::state::turns::Turn;
use crate::state::ConversationDb;
use anyhow::Result;

/// 从 tail turns 和工具历史构造 provider 历史消息。
///
/// 参数:
/// - `db`: 对话数据库
/// - `session_id`: 会话标识
/// - `turns`: checkpoint 后仍保留的轮次
///
/// 返回:
/// - provider 可直接发送的历史消息
pub(in crate::state) fn project_turn_messages_with_tool_history(
    db: &ConversationDb,
    session_id: &str,
    turns: &[Turn],
) -> Result<Vec<ChatMessage>> {
    let mut messages = Vec::new();
    for turn in turns {
        append_turn_messages(db, session_id, turn, &mut messages)?;
    }
    Ok(messages)
}

/// 追加单个轮次的 provider 消息。
///
/// 参数:
/// - `db`: 对话数据库
/// - `session_id`: 会话标识
/// - `turn`: 待投影轮次
/// - `messages`: 输出消息列表
///
/// 返回:
/// - 追加是否成功
fn append_turn_messages(
    db: &ConversationDb,
    session_id: &str,
    turn: &Turn,
    messages: &mut Vec<ChatMessage>,
) -> Result<()> {
    messages.push(ChatMessage::plain("user", turn.user_content.clone()));
    let exchanges = load_tool_exchanges_for_turn(db, session_id, &turn.turn_id)?;
    if exchanges.is_empty() {
        append_assistant_context_messages(turn, messages);
        return Ok(());
    }
    let tool_calls = exchanges
        .iter()
        .map(|exchange| ToolCall {
            id: exchange.call.provider_call_id.clone(),
            kind: "function".to_string(),
            function: ToolCallFunction {
                name: exchange.call.tool_name.clone(),
                arguments: exchange.call.arguments.clone(),
            },
        })
        .collect::<Vec<_>>();
    messages.push(ChatMessage::assistant("", Some(tool_calls)));
    for exchange in exchanges {
        messages.push(ChatMessage::tool(
            exchange.call.provider_call_id.clone(),
            tool_result_content(&exchange),
        ));
    }
    append_assistant_context_messages(turn, messages);
    Ok(())
}

/// 追加助手最终回复和旧工具报告。
///
/// 参数:
/// - `turn`: 待投影轮次
/// - `messages`: 输出消息列表
///
/// 返回:
/// - 无
fn append_assistant_context_messages(turn: &Turn, messages: &mut Vec<ChatMessage>) {
    messages.push(ChatMessage::plain(
        "assistant",
        turn.assistant_content.clone(),
    ));
    messages.extend(project_legacy_tool_report_messages(&turn.tool_reports));
}

/// 构造 provider 可见工具结果内容。
///
/// 参数:
/// - `exchange`: 工具调用交换记录
///
/// 返回:
/// - provider 可见工具结果文本
fn tool_result_content(exchange: &super::model::ToolExchangeRecord) -> String {
    if let Some(replacement) = &exchange.replacement {
        return replacement.replacement.clone();
    }
    if let Some(result) = &exchange.result {
        return result.result_preview.clone();
    }
    match exchange.call.status {
        super::model::ToolCallStatus::Interrupted => {
            "tool error: tool call was interrupted before a result was recorded. Do not retry unless the user explicitly asks.".to_string()
        }
        _ => {
            "tool error: tool result is missing from durable history. Do not retry unless the user explicitly asks.".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::tool_history::repository::{
        insert_tool_call, insert_tool_result, upsert_tool_output_replacement,
    };
    use crate::state::tool_history::schema::create_tool_history_tables;
    use crate::state::tool_history::{
        NewToolCallRecord, NewToolOutputReplacement, NewToolResultRecord,
    };
    use crate::state::turns::TurnStatus;

    fn db() -> (tempfile::TempDir, ConversationDb) {
        let temp = tempfile::tempdir().unwrap();
        let db = ConversationDb::open(temp.path()).unwrap();
        let conn = db.conn.lock().unwrap();
        create_tool_history_tables(&conn).unwrap();
        drop(conn);
        (temp, db)
    }

    fn turn() -> Turn {
        Turn {
            turn_id: "turn_1".to_string(),
            seq: 1,
            user_content: "read file".to_string(),
            user_timestamp: "2026-01-01T00:00:00Z".to_string(),
            assistant_content: "done".to_string(),
            assistant_reasoning: None,
            assistant_timestamp: Some("2026-01-01T00:00:01Z".to_string()),
            status: TurnStatus::Completed,
            tool_reports: Vec::new(),
        }
    }

    #[test]
    fn projects_tool_calls_and_reuses_replacement() {
        let (_temp, db) = db();
        insert_tool_call(
            &db,
            NewToolCallRecord {
                session_id: "default".to_string(),
                turn_id: "turn_1".to_string(),
                seq: 1,
                provider_call_id: "call_1".to_string(),
                tool_name: "read_file".to_string(),
                arguments: "{\"path\":\"a\"}".to_string(),
            },
        )
        .unwrap();
        insert_tool_result(
            &db,
            NewToolResultRecord {
                session_id: "default".to_string(),
                turn_id: "turn_1".to_string(),
                provider_call_id: "call_1".to_string(),
                ok: true,
                result_preview: "preview".to_string(),
                result_ref: Some("tool-results/call_1.txt".to_string()),
                error: None,
                original_chars: 100,
            },
        )
        .unwrap();
        upsert_tool_output_replacement(
            &db,
            NewToolOutputReplacement {
                provider_call_id: "call_1".to_string(),
                session_id: "default".to_string(),
                replacement: "stable preview".to_string(),
                original_chars: 100,
                result_ref: "tool-results/call_1.txt".to_string(),
                policy: "context_clip".to_string(),
            },
        )
        .unwrap();

        let messages = project_turn_messages_with_tool_history(&db, "default", &[turn()]).unwrap();

        assert_eq!(messages.len(), 4);
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].tool_calls.as_ref().unwrap()[0].id, "call_1");
        assert_eq!(messages[2].role, "tool");
        assert!(matches!(
            messages[2].content.as_ref(),
            Some(crate::llm::ChatContent::Text(text)) if text == "stable preview"
        ));
        assert_eq!(messages[3].role, "assistant");
    }
}
