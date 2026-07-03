use crate::config::AppConfig;
use anyhow::Result;
use std::io;

use super::form::{parse_bool_field, run_form, Field};

pub(crate) fn edit_settings(stdout: &mut io::Stdout, config: &mut AppConfig) -> Result<()> {
    let mut fields = vec![
        Field::boolean("工具启用", config.tools.enabled),
        Field::new("工具最大轮数", config.tools.max_rounds.to_string()),
        Field::boolean("渐进式工具加载", config.tools.progressive_loading_enabled),
        Field::boolean("后台命令启用", config.tools.background_commands_enabled),
        Field::new(
            "后台命令默认超时秒数",
            config.tools.background_command_timeout_seconds.to_string(),
        ),
        Field::new(
            "后台命令日志最大字节",
            config.tools.background_command_log_max_bytes.to_string(),
        ),
        Field::new(
            "后台命令停止宽限秒数",
            config
                .tools
                .background_command_stop_grace_seconds
                .to_string(),
        ),
        Field::boolean("Skills 启用", config.skills.enabled),
        Field::boolean("允许执行命令", config.skills.allow_command_execution),
        Field::new("显示思考过程", config.display.reasoning.clone())
            .choices(&["summary", "full", "hidden"]),
        Field::new("显示工具调用信息", config.display.tool_calls.clone())
            .choices(&["summary", "full", "hidden"]),
        Field::boolean("工具名可读显示", config.display.readable_tool_names),
        Field::boolean("等待动效显示模型", config.display.wait_show_model),
        Field::boolean(
            "等待动效显示思考等级",
            config.display.wait_show_thinking_level,
        ),
    ];
    if run_form(stdout, " GLOBAL SETTINGS ", &mut fields)? {
        config.tools.enabled = parse_bool_field(&fields[0].value)?;
        config.tools.max_rounds = fields[1].value.trim().parse::<usize>()?;
        config.tools.progressive_loading_enabled = parse_bool_field(&fields[2].value)?;
        config.tools.background_commands_enabled = parse_bool_field(&fields[3].value)?;
        config.tools.background_command_timeout_seconds = fields[4].value.trim().parse::<u64>()?;
        config.tools.background_command_log_max_bytes = fields[5].value.trim().parse::<u64>()?;
        config.tools.background_command_stop_grace_seconds =
            fields[6].value.trim().parse::<u64>()?;
        config.skills.enabled = parse_bool_field(&fields[7].value)?;
        config.skills.allow_command_execution = parse_bool_field(&fields[8].value)?;
        config.display.reasoning = fields[9].value.trim().to_string();
        config.display.tool_calls = fields[10].value.trim().to_string();
        config.display.readable_tool_names = parse_bool_field(&fields[11].value)?;
        config.display.wait_show_model = parse_bool_field(&fields[12].value)?;
        config.display.wait_show_thinking_level = parse_bool_field(&fields[13].value)?;
    }
    Ok(())
}
