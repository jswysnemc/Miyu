use super::model::ProviderConfig;
use super::model_units::deserialize_optional_context_chars;
use serde::{Deserialize, Serialize};

pub const MODEL_TAG_TOOL: &str = "tool";
pub const MODEL_TAG_THINKING: &str = "thinking";
pub const MODEL_TAG_VISION: &str = "vision";
pub const MODEL_TAG_WEB_SEARCH: &str = "web_search";
pub const MODEL_TAG_FAST: &str = "fast";
pub const MODEL_TAG_LOW_COST: &str = "low_cost";
pub const MODEL_TAGS: [&str; 6] = [
    MODEL_TAG_TOOL,
    MODEL_TAG_THINKING,
    MODEL_TAG_VISION,
    MODEL_TAG_WEB_SEARCH,
    MODEL_TAG_FAST,
    MODEL_TAG_LOW_COST,
];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelMetadata {
    #[serde(
        default,
        deserialize_with = "deserialize_optional_context_chars",
        skip_serializing_if = "Option::is_none"
    )]
    pub context_chars: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl ModelMetadata {
    /// 判断模型元数据是否为空。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 没有上下文长度和标签时返回 true
    pub fn is_empty(&self) -> bool {
        self.context_chars.is_none() && self.tags.is_empty()
    }
}

impl ProviderConfig {
    /// 获取模型上下文字符数。
    ///
    /// 参数:
    /// - `model`: 模型 ID
    ///
    /// 返回:
    /// - 新模型元数据中的上下文长度，或旧字段中的上下文长度
    pub fn model_context_chars_for(&self, model: &str) -> Option<usize> {
        self.model_metadata
            .get(model)
            .and_then(|metadata| metadata.context_chars)
            .or_else(|| self.model_context_chars.get(model).copied())
    }

    /// 获取模型标签。
    ///
    /// 参数:
    /// - `model`: 模型 ID
    ///
    /// 返回:
    /// - 模型标签列表
    pub fn model_tags_for(&self, model: &str) -> &[String] {
        self.model_metadata
            .get(model)
            .map(|metadata| metadata.tags.as_slice())
            .unwrap_or(&[])
    }

    /// 设置模型上下文字符数。
    ///
    /// 参数:
    /// - `model`: 模型 ID
    /// - `context_chars`: 上下文字符数，空值表示取消配置
    ///
    /// 返回:
    /// - 无
    pub fn set_model_context_chars_for(&mut self, model: &str, context_chars: Option<usize>) {
        if model.trim().is_empty() {
            return;
        }
        self.model_context_chars.remove(model);
        if let Some(context_chars) = context_chars {
            self.model_metadata_mut(model).context_chars = Some(context_chars);
        } else if let Some(metadata) = self.model_metadata.get_mut(model) {
            metadata.context_chars = None;
        }
        self.remove_empty_model_metadata(model);
    }

    /// 设置模型标签。
    ///
    /// 参数:
    /// - `model`: 模型 ID
    /// - `tags`: 已校验的标签列表
    ///
    /// 返回:
    /// - 无
    pub fn set_model_tags_for(&mut self, model: &str, tags: Vec<String>) {
        if model.trim().is_empty() {
            return;
        }
        if tags.is_empty() {
            if let Some(metadata) = self.model_metadata.get_mut(model) {
                metadata.tags.clear();
            }
        } else {
            self.model_metadata_mut(model).tags = tags;
        }
        self.remove_empty_model_metadata(model);
    }

    /// 获取可修改的模型元数据。
    ///
    /// 参数:
    /// - `model`: 模型 ID
    ///
    /// 返回:
    /// - 模型元数据引用
    fn model_metadata_mut(&mut self, model: &str) -> &mut ModelMetadata {
        self.model_metadata.entry(model.to_string()).or_default()
    }

    /// 清理空模型元数据。
    ///
    /// 参数:
    /// - `model`: 模型 ID
    ///
    /// 返回:
    /// - 无
    fn remove_empty_model_metadata(&mut self, model: &str) {
        if self
            .model_metadata
            .get(model)
            .map(ModelMetadata::is_empty)
            .unwrap_or(false)
        {
            self.model_metadata.remove(model);
        }
    }
}

/// 判断模型标签是否合法。
///
/// 参数:
/// - `tag`: 标签名称
///
/// 返回:
/// - 标签属于内置标签集合时返回 true
pub fn is_valid_model_tag(tag: &str) -> bool {
    MODEL_TAGS.contains(&tag.trim())
}

/// 返回模型标签显示文本。
///
/// 参数:
/// - `tags`: 标签列表
///
/// 返回:
/// - 逗号分隔的标签文本
pub fn format_model_tags(tags: &[String]) -> String {
    tags.join(",")
}
