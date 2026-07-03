use crate::config::AppConfig;
use anyhow::Result;
use std::io;

use super::form::{parse_bool_field, run_form, Field};

pub(crate) fn edit_settings(stdout: &mut io::Stdout, config: &mut AppConfig) -> Result<()> {
    let mut fields = vec![
        Field::boolean("工具启用", config.tools.enabled),
        Field::new("工具最大轮数", config.tools.max_rounds.to_string()),
        Field::boolean("Skills 启用", config.skills.enabled),
        Field::boolean("允许执行命令", config.skills.allow_command_execution),
        Field::new("显示思考过程", config.display.reasoning.clone())
            .choices(&["summary", "full", "hidden"]),
        Field::new("显示工具调用信息", config.display.tool_calls.clone())
            .choices(&["summary", "full", "hidden"]),
        Field::boolean("工具名可读显示", config.display.readable_tool_names),
    ];
    if run_form(stdout, " GLOBAL SETTINGS ", &mut fields)? {
        config.tools.enabled = parse_bool_field(&fields[0].value)?;
        config.tools.max_rounds = fields[1].value.trim().parse::<usize>()?;
        config.skills.enabled = parse_bool_field(&fields[2].value)?;
        config.skills.allow_command_execution = parse_bool_field(&fields[3].value)?;
        config.display.reasoning = fields[4].value.trim().to_string();
        config.display.tool_calls = fields[5].value.trim().to_string();
        config.display.readable_tool_names = parse_bool_field(&fields[6].value)?;
    }
    Ok(())
}
