use serde::{Deserialize, Serialize};

/// CLI 与 TUI 启动时采用的默认权限模式。
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultPermissionMode {
    Yolo,
    Audited,
    Plan,
}

impl DefaultPermissionMode {
    /// 返回配置文件使用的稳定字符串。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - `yolo`、`audited` 或 `plan`
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yolo => "yolo",
            Self::Audited => "audited",
            Self::Plan => "plan",
        }
    }

    /// 从配置界面字符串解析默认权限模式。
    ///
    /// 参数:
    /// - `value`: 配置字符串
    ///
    /// 返回:
    /// - 已识别模式；未知值回退到 YOLO
    pub fn parse_or_default(value: &str) -> Self {
        match value.trim() {
            "audited" => Self::Audited,
            "plan" => Self::Plan,
            _ => Self::Yolo,
        }
    }
}

impl Default for DefaultPermissionMode {
    fn default() -> Self {
        Self::Yolo
    }
}

/// 终端运行入口共用的权限配置。
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct PermissionConfig {
    /// CLI 与 TUI 未指定模式参数时采用的权限模式
    #[serde(default)]
    pub default_mode: DefaultPermissionMode,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证旧配置缺少权限字段时保持 YOLO 兼容默认值。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn permission_config_defaults_to_yolo_for_legacy_configs() {
        let config: PermissionConfig = serde_json::from_str("{}").unwrap();

        assert_eq!(config.default_mode, DefaultPermissionMode::Yolo);
    }
}
