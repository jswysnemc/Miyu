use serde_json::Value;

/// 判断后台命令工具调用是否应渲染为命令块。
///
/// 参数:
/// - `arguments`: `background_command` 工具参数
///
/// 返回:
/// - 是否为启动后台命令
pub(crate) fn is_background_command_start(arguments: &str) -> bool {
    serde_json::from_str::<Value>(arguments)
        .ok()
        .and_then(|value| {
            value
                .get("action")
                .and_then(Value::as_str)
                .map(|action| action == "start")
        })
        .unwrap_or(false)
}
