use crate::paths::MiyuPaths;

#[derive(Debug, Clone)]
pub(crate) struct WebAppState {
    pub(crate) paths: MiyuPaths,
}

impl WebAppState {
    /// 创建 Web 应用共享状态。
    ///
    /// 参数:
    /// - `paths`: Miyu 路径
    ///
    /// 返回:
    /// - Web 应用共享状态
    pub(crate) fn new(paths: &MiyuPaths) -> Self {
        Self {
            paths: paths.clone(),
        }
    }
}
