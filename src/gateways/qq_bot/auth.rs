use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::time::{Duration, Instant};

const DEFAULT_TOKEN_URL: &str = "https://bots.qq.com/app/getAppAccessToken";
const TOKEN_REFRESH_SKEW: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub(crate) struct QqBotAuthenticator {
    client: reqwest::Client,
    token_url: String,
    app_id: String,
    client_secret: String,
    cached: Option<CachedToken>,
}

#[derive(Debug)]
struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: Option<String>,
    expires_in: Option<u64>,
}

impl QqBotAuthenticator {
    /// 创建 QQ 官方机器人认证器。
    ///
    /// 参数:
    /// - `app_id`: QQ 开放平台 AppID
    /// - `client_secret`: QQ 开放平台 AppSecret
    ///
    /// 返回:
    /// - QQ 官方机器人认证器
    pub(crate) fn new(app_id: String, client_secret: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            token_url: DEFAULT_TOKEN_URL.to_string(),
            app_id,
            client_secret,
            cached: None,
        }
    }

    /// 获取 QQ OpenAPI Authorization 请求头。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - Authorization 请求头完整内容
    pub(crate) async fn authorization(&mut self) -> Result<String> {
        if let Some(token) = self.valid_cached_token() {
            return Ok(format!("QQBot {token}"));
        }
        let token = self.fetch_access_token().await?;
        let authorization = format!("QQBot {}", token.access_token);
        self.cached = Some(token);
        Ok(authorization)
    }

    /// 读取仍然有效的缓存 token。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - access token
    fn valid_cached_token(&self) -> Option<&str> {
        let cached = self.cached.as_ref()?;
        if Instant::now() + TOKEN_REFRESH_SKEW >= cached.expires_at {
            return None;
        }
        Some(cached.access_token.as_str())
    }

    /// 请求 QQ 官方 access token。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 缓存 token
    async fn fetch_access_token(&self) -> Result<CachedToken> {
        let response = self
            .client
            .post(&self.token_url)
            .json(&json!({
                "appId": self.app_id,
                "clientSecret": self.client_secret,
            }))
            .send()
            .await
            .with_context(|| "failed to request QQ access token")?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            bail!("QQ token API returned HTTP {status}: {body}");
        }
        let parsed = serde_json::from_str::<AccessTokenResponse>(&body)
            .with_context(|| format!("invalid QQ token response: {body}"))?;
        let access_token = parsed
            .access_token
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("QQ token response has no access_token"))?;
        let expires_in = parsed.expires_in.unwrap_or(7200).max(120);
        Ok(CachedToken {
            access_token,
            expires_at: Instant::now() + Duration::from_secs(expires_in),
        })
    }
}
