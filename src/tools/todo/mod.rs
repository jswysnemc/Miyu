mod reminder;
mod store;

use super::{ToolRegistry, ToolSpec};
use crate::i18n::text as t;
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

pub(crate) use reminder::TodoReminder;
pub(crate) use store::{TodoItem, TodoStatus, TodoStore};

/// 注册会话级 TODO 工具。
///
/// 参数:
/// - `registry`: 当前交互式会话工具注册表
/// - `file`: 当前会话 TODO 状态文件
///
/// 返回:
/// - 无
pub(crate) fn register(registry: &mut ToolRegistry, file: PathBuf) {
    registry.register(
        ToolSpec::new(
            "todo",
            t(
                "Track a multi-step plan for the current session. Actions: list reads all items; add appends a new pending item; update changes an item's text or status by id; remove deletes an item by id. Status flows pending -> in_progress -> completed (or cancelled). Rule: items advance in order, so you must finish earlier items before marking a later one in_progress or completed. Keep exactly one item in_progress at a time. Use this for tasks with three or more steps; skip it for trivial single-step work.",
                "跟踪当前会话的多步计划。动作：list 读取全部条目；add 追加一个待处理条目；update 按 id 修改条目文本或状态；remove 按 id 删除条目。状态流转为 pending -> in_progress -> completed（或 cancelled）。规则：条目按顺序推进，必须先完成前面的条目，才能把后面的条目标记为 in_progress 或 completed；同一时刻只保留一个 in_progress 条目。任务达到三步及以上时使用，单步琐碎任务不必使用。",
            ),
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "add", "update", "remove"],
                        "description": t(
                            "Which operation to perform.",
                            "要执行的操作。",
                        )
                    },
                    "id": {
                        "type": "string",
                        "description": t(
                            "Item id, required for update and remove. Obtain it from list or from the add result.",
                            "条目 id，update 与 remove 必填，可从 list 或 add 的返回结果获取。",
                        )
                    },
                    "text": {
                        "type": "string",
                        "description": t(
                            "Item text. Required for add; optional for update to rename.",
                            "条目文本。add 必填；update 时可选，用于重命名。",
                        )
                    },
                    "status": {
                        "type": "string",
                        "enum": ["pending", "in_progress", "completed", "cancelled"],
                        "description": t(
                            "New status for update. Advance items in order and keep one in_progress at a time.",
                            "update 的新状态。按顺序推进，同时只保留一个 in_progress。",
                        )
                    }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
            move |args| {
                let store = TodoStore::new(file.clone());
                async move { execute(args, store) }
            },
        )
        .writes(),
    );
}

/// 执行 TODO 工具动作。
///
/// 参数:
/// - `args`: 工具参数
/// - `store`: 当前会话 TODO 存储
///
/// 返回:
/// - JSON 格式执行结果
fn execute(args: Value, store: TodoStore) -> Result<String> {
    let action = args
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let result = match action {
        "list" => json!({"ok": true, "items": store.list()?}),
        "add" => {
            let text = string_arg(&args, "text")?;
            json!({"ok": true, "item": store.add(text)?})
        }
        "update" => {
            let id = string_arg(&args, "id")?;
            let text = args.get("text").and_then(Value::as_str);
            let status = args
                .get("status")
                .and_then(Value::as_str)
                .map(TodoStatus::parse)
                .transpose()?;
            json!({"ok": true, "item": store.update(id, text, status)?})
        }
        "remove" => {
            let id = string_arg(&args, "id")?;
            json!({"ok": true, "item": store.remove(id)?})
        }
        _ => bail!("unsupported todo action: {action}"),
    };
    Ok(result.to_string())
}

/// 读取必填字符串参数。
///
/// 参数:
/// - `args`: 工具参数
/// - `name`: 参数名称
///
/// 返回:
/// - 非空字符串参数
fn string_arg<'value>(args: &'value Value, name: &str) -> Result<&'value str> {
    let value = args
        .get(name)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if value.is_empty() {
        bail!("{name} is required")
    }
    Ok(value)
}

/// 判断工具调用是否会修改 TODO 清单。
///
/// 参数:
/// - `arguments`: 工具调用 JSON 参数
///
/// 返回:
/// - 是否属于修改动作
pub(crate) fn is_mutating_call(arguments: &str) -> bool {
    serde_json::from_str::<Value>(arguments)
        .ok()
        .and_then(|args| {
            args.get("action")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .is_some_and(|action| matches!(action.as_str(), "add" | "update" | "remove"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证工具支持新增、更新、列表和删除。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[tokio::test]
    async fn tool_supports_crud_actions() {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = ToolRegistry::new();
        register(&mut registry, dir.path().join("todos.json"));

        let added = registry
            .call("todo", r#"{"action":"add","text":"first"}"#)
            .await
            .unwrap();
        let id = serde_json::from_str::<Value>(&added).unwrap()["item"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        registry
            .call(
                "todo",
                &json!({"action":"update","id":id,"status":"in_progress"}).to_string(),
            )
            .await
            .unwrap();
        registry
            .call(
                "todo",
                &json!({"action":"update","id":id,"status":"completed"}).to_string(),
            )
            .await
            .unwrap();
        let listed = registry.call("todo", r#"{"action":"list"}"#).await.unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&listed).unwrap()["items"][0]["status"],
            "completed"
        );
        registry
            .call("todo", &json!({"action":"remove","id":id}).to_string())
            .await
            .unwrap();
        let listed = registry.call("todo", r#"{"action":"list"}"#).await.unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&listed).unwrap()["items"],
            json!([])
        );
    }
}
