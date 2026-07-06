use super::*;

pub(super) fn run_reset(paths: &MiyuPaths, scope: Option<&str>) -> Result<()> {
    let all = match scope {
        None => false,
        Some("all") => true,
        Some("全部") => true,
        Some(scope) => bail!("{}: {scope}", t("unknown reset scope", "未知 reset 范围")),
    };
    println!("{}", crate::control_commands::clear_state(paths, all)?);
    Ok(())
}
