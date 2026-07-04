use crate::llm::ToolDefinition;
use crate::tools::{self, ToolRegistry};
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub(crate) struct ToolVisibility {
    progressive: bool,
    loaded: BTreeSet<String>,
}

impl ToolVisibility {
    /// 创建工具可见性状态。
    ///
    /// 参数:
    /// - `progressive`: 是否启用渐进式工具加载
    ///
    /// 返回:
    /// - 新的工具可见性状态
    pub(crate) fn new(progressive: bool) -> Self {
        Self {
            progressive,
            loaded: BTreeSet::new(),
        }
    }

    /// 计算当前应暴露给模型的工具定义。
    ///
    /// 参数:
    /// - `registry`: 完整工具注册表
    ///
    /// 返回:
    /// - 当前可见的工具定义列表
    pub(crate) fn definitions(&self, registry: &ToolRegistry) -> Vec<ToolDefinition> {
        if !self.progressive {
            return registry.definitions();
        }
        let names = registry
            .tool_infos()
            .into_iter()
            .filter(|info| self.is_visible(&info.name))
            .map(|info| info.name)
            .collect::<BTreeSet<_>>();
        registry.definitions_for_names(&names)
    }

    /// 判断工具当前是否允许被模型调用。
    ///
    /// 参数:
    /// - `name`: 工具名称
    ///
    /// 返回:
    /// - 当前是否可见并允许调用
    pub(crate) fn is_visible(&self, name: &str) -> bool {
        !self.progressive || tools::progressive::is_initial_tool(name) || self.loaded.contains(name)
    }

    /// 判断当前工具调用是否为加载工具调用。
    ///
    /// 参数:
    /// - `name`: 工具名称
    ///
    /// 返回:
    /// - 是否为 `load_tools`
    pub(crate) fn is_loader_call(&self, name: &str) -> bool {
        self.progressive && name == tools::LOAD_TOOLS_NAME
    }

    /// 恢复已经加载过的工具集合。
    ///
    /// 参数:
    /// - `registry`: 当前完整工具注册表
    /// - `names`: 上一轮保存的已加载工具名称
    ///
    /// 返回:
    /// - 无
    pub(crate) fn restore_loaded_tools(&mut self, registry: &ToolRegistry, names: &[String]) {
        self.loaded.clear();
        if !self.progressive {
            return;
        }
        for name in names {
            if registry.contains(name) && !tools::progressive::is_initial_tool(name) {
                self.loaded.insert(name.clone());
            }
        }
    }

    /// 获取已经额外加载的工具名称。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 已加载工具名称列表
    pub(crate) fn loaded_tool_names(&self) -> Vec<String> {
        self.loaded.iter().cloned().collect()
    }

    /// 按加载工具参数更新可见工具集合。
    ///
    /// 参数:
    /// - `registry`: 完整工具注册表
    /// - `arguments`: `load_tools` 的 JSON 参数
    ///
    /// 返回:
    /// - 给模型的加载结果说明
    pub(crate) fn load_from_arguments(
        &mut self,
        registry: &ToolRegistry,
        arguments: &str,
    ) -> Result<String> {
        let args = if arguments.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str::<Value>(arguments)?
        };
        let tool_name = string_arg(&args, "tool_name");
        let group_name = string_arg(&args, "group_name");
        if tool_name.is_some() == group_name.is_some() {
            bail!("provide exactly one of tool_name or group_name");
        }
        let loaded = if let Some(tool_name) = tool_name {
            self.load_tool(registry, &tool_name)?
        } else {
            self.load_group(registry, &group_name.unwrap())?
        };
        Ok(serde_json::to_string_pretty(&json!({
            "ok": true,
            "loaded_tools": loaded,
            "visible_tools": self.visible_tool_names(registry),
        }))?)
    }

    /// 加载单个工具。
    ///
    /// 参数:
    /// - `registry`: 完整工具注册表
    /// - `name`: 要加载的工具名称
    ///
    /// 返回:
    /// - 本次请求加载的工具名称列表
    fn load_tool(&mut self, registry: &ToolRegistry, name: &str) -> Result<Vec<String>> {
        if !registry.contains(name) {
            bail!("unknown tool: {name}");
        }
        if name == tools::LOAD_TOOLS_NAME {
            return Ok(vec![name.to_string()]);
        }
        self.loaded.insert(name.to_string());
        Ok(vec![name.to_string()])
    }

    /// 加载一个用途分组下的所有工具。
    ///
    /// 参数:
    /// - `registry`: 完整工具注册表
    /// - `group`: 要加载的分组名称
    ///
    /// 返回:
    /// - 本次请求加载的工具名称列表
    fn load_group(&mut self, registry: &ToolRegistry, group: &str) -> Result<Vec<String>> {
        let names = registry
            .tool_infos()
            .into_iter()
            .filter(|info| {
                info.name != tools::LOAD_TOOLS_NAME
                    && tools::progressive::tool_group(&info.name) == group
            })
            .map(|info| info.name)
            .collect::<Vec<_>>();
        if names.is_empty() {
            bail!("unknown or empty tool group: {group}");
        }
        for name in &names {
            self.loaded.insert(name.clone());
        }
        Ok(names)
    }

    /// 获取当前可见工具名称。
    ///
    /// 参数:
    /// - `registry`: 完整工具注册表
    ///
    /// 返回:
    /// - 当前可见工具名称列表
    fn visible_tool_names(&self, registry: &ToolRegistry) -> Vec<String> {
        registry
            .tool_infos()
            .into_iter()
            .filter(|info| self.is_visible(&info.name))
            .map(|info| info.name)
            .collect()
    }
}

/// 从 JSON 参数中读取非空字符串。
///
/// 参数:
/// - `args`: JSON 参数对象
/// - `name`: 字段名
///
/// 返回:
/// - 字段存在且非空时返回字符串
fn string_arg(args: &Value, name: &str) -> Option<String> {
    args.get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{self, ToolSpec};
    use serde_json::json;

    #[test]
    fn progressive_visibility_starts_with_base_and_loader() {
        let mut registry = test_registry();
        tools::register_progressive_loader(&mut registry);
        let visibility = ToolVisibility::new(true);
        let names = definition_names(visibility.definitions(&registry));

        assert!(names.contains(&"read_file".to_string()));
        assert!(names.contains(&tools::LOAD_TOOLS_NAME.to_string()));
        assert!(names.contains(&"load_skill".to_string()));
        assert!(!names.contains(&"web_search".to_string()));
    }

    #[test]
    fn progressive_visibility_loads_group() {
        let mut registry = test_registry();
        tools::register_progressive_loader(&mut registry);
        let mut visibility = ToolVisibility::new(true);

        visibility
            .load_from_arguments(&registry, r#"{"group_name":"web"}"#)
            .unwrap();
        let names = definition_names(visibility.definitions(&registry));

        assert!(names.contains(&"web_search".to_string()));
        assert!(!names.contains(&"analyze_image".to_string()));
    }

    #[test]
    fn progressive_visibility_restores_loaded_tools() {
        let mut registry = test_registry();
        tools::register_progressive_loader(&mut registry);
        let mut visibility = ToolVisibility::new(true);

        visibility.restore_loaded_tools(
            &registry,
            &[
                "web_search".to_string(),
                "unknown_tool".to_string(),
                "read_file".to_string(),
            ],
        );
        let names = definition_names(visibility.definitions(&registry));

        assert!(names.contains(&"web_search".to_string()));
        assert!(names.contains(&"read_file".to_string()));
        assert!(!names.contains(&"unknown_tool".to_string()));
        assert_eq!(
            visibility.loaded_tool_names(),
            vec!["web_search".to_string()]
        );
    }

    fn test_registry() -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        registry.register(ToolSpec::new(
            "read_file",
            "Read a file.",
            json!({"type":"object","properties":{},"additionalProperties":false}),
            |_| async { Ok("ok".to_string()) },
        ));
        registry.register(ToolSpec::new(
            "web_search",
            "Search the web.",
            json!({"type":"object","properties":{},"additionalProperties":false}),
            |_| async { Ok("ok".to_string()) },
        ));
        registry.register(ToolSpec::new(
            "analyze_image",
            "Analyze an image.",
            json!({"type":"object","properties":{},"additionalProperties":false}),
            |_| async { Ok("ok".to_string()) },
        ));
        registry.register(ToolSpec::new(
            "load_skill",
            "Load a skill.",
            json!({"type":"object","properties":{},"additionalProperties":false}),
            |_| async { Ok("ok".to_string()) },
        ));
        registry
    }

    fn definition_names(definitions: Vec<ToolDefinition>) -> Vec<String> {
        definitions
            .into_iter()
            .map(|definition| definition.function.name)
            .collect()
    }
}
