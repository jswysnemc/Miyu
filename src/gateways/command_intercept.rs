use crate::control_commands::{parse_control_command, ControlCommand, ControlSurface};
use crate::i18n::text as t;
use crate::paths::MiyuPaths;
use anyhow::Result;

/// 处理 gateway 入站控制命令。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `prompt`: 入站消息文本
///
/// 返回:
/// - 已处理时返回需要发送的回复文本，否则返回空
pub(crate) async fn handle_gateway_command(
    paths: &MiyuPaths,
    prompt: &str,
) -> Result<Option<String>> {
    if is_gateway_clear_all_command(prompt) {
        let message = crate::control_commands::clear_state(paths, true)?;
        return Ok(Some(format!(
            "{message}；{}",
            t("gateway is still running", "gateway 仍在运行")
        )));
    }
    let Some(command) = parse_control_command(prompt, ControlSurface::Gateway)? else {
        return Ok(None);
    };
    Ok(Some(match command {
        ControlCommand::Help => crate::control_commands::help_text(ControlSurface::Gateway),
        ControlCommand::New { title } => {
            crate::control_commands::create_new_session(paths, &title)?
        }
        ControlCommand::Resume { id } => match id {
            Some(id) => crate::control_commands::resume_session(paths, &id)?,
            None => {
                // 网关无交互 UI，列出会话并提示带 ID 调用
                let choices = crate::control_commands::session_resume_choices(paths)?;
                let mut lines = vec![t(
                    "Provide /resume <id>. Available sessions:",
                    "请使用 /resume <id>。可用会话：",
                )
                .to_string()];
                for (_, label) in choices {
                    lines.push(label);
                }
                lines.join("\n")
            }
        },
        ControlCommand::Compact { keep_tail_turns } => {
            crate::control_commands::compact_conversation_from_paths(paths, keep_tail_turns).await?
        }
        ControlCommand::Clear { all } => crate::control_commands::clear_state(paths, all)?,
        ControlCommand::Model { selection } => {
            crate::control_commands::run_model_command(paths, selection, ControlSurface::Gateway)?
                .message
        }
    }))
}

/// 判断入站消息是否是 gateway 安全清空命令。
///
/// 参数:
/// - `prompt`: 入站消息文本
///
/// 返回:
/// - 是否是 `miyu clear all`
fn is_gateway_clear_all_command(prompt: &str) -> bool {
    let words = prompt.split_whitespace().collect::<Vec<_>>();
    if words.len() != 3 {
        return false;
    }
    let executable = words[0].trim_matches(['`', '"', '\'']);
    let command = words[1].trim_matches(['`', '"', '\'']);
    let scope = words[2].trim_matches(['`', '"', '\'']);
    executable.ends_with("miyu") && command == "clear" && scope == "all"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::paths::MiyuPaths;
    use crate::state::StateStore;
    use std::path::PathBuf;

    fn test_paths(root: PathBuf) -> MiyuPaths {
        MiyuPaths {
            config_dir: root.join("config"),
            config_file: root.join("config/config.jsonc"),
            secrets_file: root.join("config/secrets.jsonc"),
            skills_dir: root.join("config/skills"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            state_dir: root.join("state"),
            pictures_dir: root.join("pictures"),
            fish_hook_file: root.join("fish/miyu.fish"),
            bash_hook_file: root.join("shell/bash-hook.sh"),
            zsh_hook_file: root.join("shell/zsh-hook.zsh"),
            powershell_hook_file: root.join("shell/powershell-hook.ps1"),
        }
    }

    #[test]
    fn detects_clear_all_command() {
        assert!(is_gateway_clear_all_command("miyu clear all"));
        assert!(is_gateway_clear_all_command("`miyu` `clear` `all`"));
        assert!(is_gateway_clear_all_command("/usr/bin/miyu clear all"));
        assert!(!is_gateway_clear_all_command("miyu reset all"));
        assert!(!is_gateway_clear_all_command("miyu clear"));
        assert!(!is_gateway_clear_all_command("please miyu clear all"));
    }

    #[tokio::test]
    async fn gateway_clear_all_clears_state_without_agent_turn() {
        let temp = tempfile::tempdir().unwrap();
        let paths = test_paths(temp.path().to_path_buf());
        AppConfig::init_files(&paths).unwrap();
        let state = StateStore::new(&paths).unwrap();
        state.start_turn("turn_1", "hello").unwrap();
        state.complete_turn("turn_1", "hi", None).unwrap();

        let reply = handle_gateway_command(&paths, "miyu clear all")
            .await
            .unwrap();

        assert!(reply.unwrap().contains("gateway"));
        assert!(state.load_conversation().unwrap().is_empty());
    }

    #[tokio::test]
    async fn gateway_help_supports_english_and_chinese_slash_commands() {
        let temp = tempfile::tempdir().unwrap();
        let paths = test_paths(temp.path().to_path_buf());

        let english = handle_gateway_command(&paths, "/help").await.unwrap();
        let chinese = handle_gateway_command(&paths, "/帮助").await.unwrap();

        assert!(english.unwrap().contains("/compact"));
        assert!(chinese.unwrap().contains("/压缩"));
    }
}
