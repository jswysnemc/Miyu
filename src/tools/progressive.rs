use super::groups::{group_description, group_for_tool, is_base_tool};
use super::{ToolPermission, ToolRegistry, ToolSpec};
use serde_json::json;
use std::collections::BTreeMap;

pub(crate) const LOAD_TOOLS_NAME: &str = "load_tools";

/// 注册渐进式工具加载器。
///
/// 参数:
/// - `registry`: 已注册完整工具处理器的工具注册表
///
/// 返回:
/// - 无
pub(crate) fn register_loader(registry: &mut ToolRegistry) {
    let description = loader_description(registry);
    registry.register(ToolSpec::new(
        LOAD_TOOLS_NAME,
        description,
        json!({
            "type": "object",
            "properties": {
                "tool_name": {
                    "type": "string",
                    "description": "Name of one tool to load. Use either tool_name or group_name."
                },
                "group_name": {
                    "type": "string",
                    "description": "Name of one tool group to load. Use either tool_name or group_name."
                }
            },
            "additionalProperties": false
        }),
        |_| async move {
            Ok("工具加载请求已收到。后续工具可见性由对话运行时更新。".to_string())
        },
    ));
}

/// 判断工具是否应在渐进式模式启动时默认可见。
///
/// 参数:
/// - `name`: 工具名称
///
/// 返回:
/// - 是否为默认可见工具
pub(crate) fn is_initial_tool(name: &str) -> bool {
    name == LOAD_TOOLS_NAME || name == "load_skill" || is_base_tool(name)
}

/// 获取工具所属用途分组。
///
/// 参数:
/// - `name`: 工具名称
///
/// 返回:
/// - 用途分组名称
pub(crate) fn tool_group(name: &str) -> &'static str {
    group_for_tool(name)
}

/// 生成加载工具描述。
///
/// 参数:
/// - `registry`: 已注册完整工具处理器的工具注册表
///
/// 返回:
/// - 包含可加载工具名和分组的工具描述
fn loader_description(registry: &ToolRegistry) -> String {
    let mut groups: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for info in registry.tool_infos() {
        if is_base_tool(&info.name) || info.name == LOAD_TOOLS_NAME {
            continue;
        }
        let permission = match info.permission {
            ToolPermission::ReadOnly => "read",
            ToolPermission::Writes => "write",
        };
        let summary = info
            .description
            .split(['.', '。'])
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("no description");
        groups
            .entry(group_for_tool(&info.name))
            .or_default()
            .push(format!("{} ({permission}) - {summary}", info.name));
    }
    let mut text = String::from(
        "Load additional tools before using them. Use tool_name to load one tool, or group_name to load an entire group. Initial tools are limited to base tools and this loader.\n\nAvailable groups:\n",
    );
    for (group, names) in groups {
        text.push_str(&format!(
            "- {group}: {}. Tools: {}\n",
            group_description(group),
            names.join(", ")
        ));
    }
    text
}
