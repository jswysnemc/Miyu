use super::*;

pub(super) fn run_reset(paths: &MiyuPaths, scope: Option<&str>) -> Result<()> {
    let all = match scope {
        None => false,
        Some("all") => true,
        Some(scope) => bail!("{}: {scope}", t("unknown reset scope", "未知 reset 范围")),
    };
    let config = AppConfig::load_or_default(paths)?;
    StateStore::new(paths)?.reset_conversation()?;
    let memory = MemoryStore::new(&config, paths);
    if all {
        memory.reset_all(false)?;
    } else {
        memory.clear_evicted_context()?;
        memory.clear_pending_events()?;
    }
    tools::clear_aur_review_state(paths)?;
    println!(
        "{}",
        if all {
            t(
                "cleared current conversation history and all memory",
                "已清空当前会话历史与全部记忆",
            )
        } else {
            t("cleared current conversation history", "已清空当前会话历史")
        }
    );
    Ok(())
}
