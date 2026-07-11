mod background;
mod background_actions;
mod background_runtime;
mod background_schema;
mod background_tasks;
pub(crate) mod background_timeout;
mod command_guard;
mod process;
mod run;
mod store;

use crate::config::AppConfig;
use crate::paths::MiyuPaths;
use crate::tools::ToolRegistry;

use background_tasks::BackgroundRuntimeOwner;

pub(crate) use background::{
    cleanup_background_tasks_for_user, list_background_tasks_for_user,
    read_background_task_output_for_user, start_background_task_for_user,
    start_gateway_background_task_for_user, stop_background_task_for_user,
};

/// 注册命令相关工具。
///
/// 参数:
/// - `registry`: 工具注册表
/// - `config`: 应用配置
/// - `paths`: Miyu 路径
/// - `allow_command_execution`: 是否允许执行写入类命令
pub(crate) fn register(
    registry: &mut ToolRegistry,
    config: &AppConfig,
    paths: &MiyuPaths,
    allow_command_execution: bool,
) {
    run::register(registry, config, allow_command_execution);
    if config.tools.background_commands_enabled {
        background::register(
            registry,
            config.clone(),
            paths.clone(),
            allow_command_execution,
        );
    }
}

/// 为命令模式重注册后台命令工具。
///
/// 参数:
/// - `registry`: 工具注册表
/// - `config`: 应用配置
/// - `paths`: Miyu 路径
/// - `session_id`: 会话标识
pub(crate) fn register_command_mode_background(
    registry: &mut ToolRegistry,
    config: &AppConfig,
    paths: &MiyuPaths,
    session_id: &str,
) {
    if config.tools.background_commands_enabled {
        background::register_with_runtime_owner(
            registry,
            config.clone(),
            paths.clone(),
            true,
            Some(BackgroundRuntimeOwner::command_mode(session_id)),
        );
    }
}

/// 注册只读命令相关工具。
///
/// 参数:
/// - `registry`: 工具注册表
/// - `config`: 应用配置
/// - `paths`: Miyu 路径
pub(crate) fn register_readonly(
    registry: &mut ToolRegistry,
    config: &AppConfig,
    paths: &MiyuPaths,
) {
    run::register_readonly(registry, config);
    if config.tools.background_commands_enabled {
        background::register_readonly(registry, config.clone(), paths.clone());
    }
}
