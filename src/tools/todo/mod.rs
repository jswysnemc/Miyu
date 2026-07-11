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
                "Manage the current session todo list. Use list to inspect it, add to create an item, update to change text or status, and remove to delete an item. Status values: pending, in_progress, completed, cancelled.",
                "管理当前会话的 TODO 清单。使用 list 查看，add 新增，update 修改内容或状态，remove 删除。状态可用 pending、in_progress、completed、cancelled。",
            ),
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "add", "update", "remove"]
                    },
                    "id": { "type": "string", "description": t("Todo item id.", "TODO 项标识。") },
                    "text": { "type": "string", "description": t("Todo item text.", "TODO 项内容。") },
                    "status": {
                        "type": "string",
                        "enum": ["pending", "in_progress", "completed", "cancelled"]
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
