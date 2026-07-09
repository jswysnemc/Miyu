use crate::config::AppConfig;
use crate::i18n::text as t;
use anyhow::Result;
use std::io;

use super::form::{parse_bool_field, run_form, Field};

pub(crate) fn edit_settings(stdout: &mut io::Stdout, config: &mut AppConfig) -> Result<()> {
    let mut fields = vec![
        Field::boolean(t("Tools enabled", "工具启用"), config.tools.enabled),
        Field::new(
            t("Tool max rounds", "工具最大轮数"),
            config.tools.max_rounds.to_string(),
        ),
        Field::new(
            t(
                "Command shell, empty uses user shell",
                "命令执行 Shell，留空使用用户 Shell",
            ),
            config.tools.command_shell.clone(),
        ),
        Field::boolean(
            t("Progressive tool loading", "渐进式工具加载"),
            config.tools.progressive_loading_enabled,
        ),
        Field::boolean(
            t("Background commands enabled", "后台命令启用"),
            config.tools.background_commands_enabled,
        ),
        Field::new(
            t(
                "Background command default timeout seconds, 0 means no timeout",
                "后台命令默认超时秒数，0 表示不超时",
            ),
            config.tools.background_command_timeout_seconds.to_string(),
        ),
        Field::new(
            t("Background command max log bytes", "后台命令日志最大字节"),
            config.tools.background_command_log_max_bytes.to_string(),
        ),
        Field::new(
            t(
                "Background command stop grace seconds",
                "后台命令停止宽限秒数",
            ),
            config
                .tools
                .background_command_stop_grace_seconds
                .to_string(),
        ),
        Field::boolean(t("Skills enabled", "Skills 启用"), config.skills.enabled),
        Field::boolean(
            t("Allow command execution", "允许执行命令"),
            config.skills.allow_command_execution,
        ),
        Field::new(
            t("Show reasoning", "显示思考过程"),
            config.display.reasoning.clone(),
        )
        .choices(&["summary", "full", "hidden"]),
        Field::new(
            t("Show tool call information", "显示工具调用信息"),
            config.display.tool_calls.clone(),
        )
        .choices(&["summary", "full", "hidden"]),
        Field::boolean(
            t("Readable tool names", "工具名可读显示"),
            config.display.readable_tool_names,
        ),
        Field::boolean(
            t("Show model in wait animation", "等待动效显示模型"),
            config.display.wait_show_model,
        ),
        Field::boolean(
            t(
                "Show thinking level in wait animation",
                "等待动效显示思考等级",
            ),
            config.display.wait_show_thinking_level,
        ),
    ];
    if run_form(
        stdout,
        t(" GLOBAL SETTINGS ", " 全局参数设置 "),
        &mut fields,
    )? {
        config.tools.enabled = parse_bool_field(&fields[0].value)?;
        config.tools.max_rounds = fields[1].value.trim().parse::<usize>()?;
        config.tools.command_shell = fields[2].value.trim().to_string();
        config.tools.progressive_loading_enabled = parse_bool_field(&fields[3].value)?;
        config.tools.background_commands_enabled = parse_bool_field(&fields[4].value)?;
        config.tools.background_command_timeout_seconds = fields[5].value.trim().parse::<u64>()?;
        config.tools.background_command_log_max_bytes = fields[6].value.trim().parse::<u64>()?;
        config.tools.background_command_stop_grace_seconds =
            fields[7].value.trim().parse::<u64>()?;
        config.skills.enabled = parse_bool_field(&fields[8].value)?;
        config.skills.allow_command_execution = parse_bool_field(&fields[9].value)?;
        config.display.reasoning = fields[10].value.trim().to_string();
        config.display.tool_calls = fields[11].value.trim().to_string();
        config.display.readable_tool_names = parse_bool_field(&fields[12].value)?;
        config.display.wait_show_model = parse_bool_field(&fields[13].value)?;
        config.display.wait_show_thinking_level = parse_bool_field(&fields[14].value)?;
    }
    Ok(())
}
