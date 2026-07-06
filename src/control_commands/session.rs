use crate::i18n::text as t;
use crate::paths::MiyuPaths;
use anyhow::Result;

/// 创建并切换到新会话。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `title`: 可选会话标题文本
///
/// 返回:
/// - 创建结果文本
pub fn create_new_session(paths: &MiyuPaths, title: &str) -> Result<String> {
    let title = title.trim();
    let session = if title.is_empty() {
        crate::state::create_session(paths, None)?
    } else {
        crate::state::create_session(paths, Some(title))?
    };
    Ok(format!(
        "{}: {}  {}",
        t("created session", "已创建会话"),
        session.id,
        session.title
    ))
}
