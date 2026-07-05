use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug)]
pub(crate) struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    /// 创建内部错误响应。
    ///
    /// 参数:
    /// - `err`: 原始错误
    ///
    /// 返回:
    /// - API 错误
    pub(crate) fn internal(err: impl std::fmt::Display) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: err.to_string(),
        }
    }

    /// 创建请求错误响应。
    ///
    /// 参数:
    /// - `message`: 错误消息
    ///
    /// 返回:
    /// - API 错误
    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    /// 转换为 HTTP 响应。
    ///
    /// 参数:
    /// - `self`: API 错误
    ///
    /// 返回:
    /// - HTTP 响应
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "ok": false,
                "error": self.message,
            })),
        )
            .into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    /// 从 anyhow 错误转换为 API 错误。
    ///
    /// 参数:
    /// - `err`: 原始错误
    ///
    /// 返回:
    /// - API 错误
    fn from(err: anyhow::Error) -> Self {
        Self::internal(err)
    }
}

impl From<serde_json::Error> for ApiError {
    /// 从 JSON 错误转换为 API 错误。
    ///
    /// 参数:
    /// - `err`: JSON 错误
    ///
    /// 返回:
    /// - API 错误
    fn from(err: serde_json::Error) -> Self {
        Self::internal(err)
    }
}

pub(crate) type ApiResult<T> = Result<T, ApiError>;
