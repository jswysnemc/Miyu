use crate::config::{AppConfig, ProviderConfig};
use anyhow::{bail, Result};
use crossterm::cursor::MoveTo;
use crossterm::event::KeyCode;
use crossterm::queue;
use crossterm::style::Print;
use crossterm::terminal::{self, Clear, ClearType};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use super::form::{parse_bool_field, run_form, Field};
use super::input::{read_key, read_key_with_timeout};
use super::ui::{draw_column, draw_menu, message, truncate};

pub(crate) struct ProviderBrowser<'a> {
    config: &'a mut AppConfig,
    active_col: usize,
    provider_idx: usize,
    org_idx: usize,
    model_idx: usize,
    filter: String,
    filter_mode: bool,
    raw_models: Vec<String>,
    orgs: Vec<String>,
    models: Vec<ModelEntry>,
    status: String,
    loading: bool,
    fetch_seq: u64,
    fetch_rx: Option<Receiver<FetchResult>>,
}

impl<'a> ProviderBrowser<'a> {
    pub(crate) fn new(config: &'a mut AppConfig) -> Self {
        Self {
            config,
            active_col: 0,
            provider_idx: 0,
            org_idx: 0,
            model_idx: 0,
            filter: String::new(),
            filter_mode: false,
            raw_models: Vec::new(),
            orgs: Vec::new(),
            models: Vec::new(),
            status: String::new(),
            loading: false,
            fetch_seq: 0,
            fetch_rx: None,
        }
    }

    pub(crate) fn run(mut self, stdout: &mut io::Stdout) -> Result<()> {
        self.refresh_models();
        loop {
            self.poll_fetch_result();
            self.draw(stdout)?;
            match read_key_with_timeout(if self.loading {
                Some(Duration::from_millis(100))
            } else {
                None
            })? {
                None => continue,
                Some(key) => match key {
                    key if self.filter_mode => self.handle_filter_key(key),
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Left | KeyCode::Char('h') => self.move_left(),
                    KeyCode::Right | KeyCode::Char('l') => self.move_right(),
                    KeyCode::Up | KeyCode::Char('k') => self.move_up(),
                    KeyCode::Down | KeyCode::Char('j') => self.move_down(),
                    KeyCode::Char('/') => {
                        self.filter_mode = true;
                        self.filter.clear();
                        self.rebuild_models();
                    }
                    KeyCode::Char('r') => self.refresh_models(),
                    KeyCode::Char('a') => self.add_provider(stdout)?,
                    KeyCode::Char('d') => self.delete_provider(),
                    KeyCode::Tab if self.active_col == 2 => self.toggle_model_activation(),
                    KeyCode::Enter | KeyCode::Char('i') => self.select_or_edit(stdout)?,
                    _ => {}
                },
            }
        }
    }

    fn handle_filter_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.filter_mode = false;
                self.filter.clear();
            }
            KeyCode::Enter => self.filter_mode = false,
            KeyCode::Backspace => {
                self.filter.pop();
            }
            KeyCode::Char(ch) => self.filter.push(ch),
            _ => {}
        }
        self.rebuild_models();
    }

    fn move_left(&mut self) {
        self.active_col = self.active_col.saturating_sub(1);
    }

    fn move_right(&mut self) {
        self.active_col = (self.active_col + 1).min(2);
    }

    fn move_up(&mut self) {
        match self.active_col {
            0 => {
                self.provider_idx = self.provider_idx.saturating_sub(1);
                self.refresh_models();
            }
            1 => {
                self.org_idx = self.org_idx.saturating_sub(1);
                self.rebuild_models();
            }
            2 => self.model_idx = self.model_idx.saturating_sub(1),
            _ => {}
        }
    }

    fn move_down(&mut self) {
        match self.active_col {
            0 => {
                self.provider_idx =
                    (self.provider_idx + 1).min(self.config.providers.len().saturating_sub(1));
                self.refresh_models();
            }
            1 => {
                self.org_idx = (self.org_idx + 1).min(self.orgs.len().saturating_sub(1));
                self.rebuild_models();
            }
            2 => self.model_idx = (self.model_idx + 1).min(self.models.len().saturating_sub(1)),
            _ => {}
        }
    }

    fn refresh_models(&mut self) {
        self.provider_idx = self
            .provider_idx
            .min(self.config.providers.len().saturating_sub(1));
        self.raw_models.clear();
        self.orgs = vec!["All".to_string()];
        self.models.clear();
        self.fetch_seq += 1;
        if let Some(provider) = self.config.providers.get(self.provider_idx).cloned() {
            let seq = self.fetch_seq;
            let (tx, rx) = mpsc::channel();
            self.fetch_rx = Some(rx);
            self.loading = true;
            self.status = "正在获取模型列表...".to_string();
            std::thread::spawn(move || {
                let result = fetch_models(&provider).map_err(|err| err.to_string());
                let _ = tx.send((seq, result));
            });
        } else {
            self.fetch_rx = None;
            self.loading = false;
            self.status.clear();
        }
        self.org_idx = 0;
        self.model_idx = 0;
    }

    fn poll_fetch_result(&mut self) {
        let Some(rx) = &self.fetch_rx else {
            return;
        };
        let Ok((seq, result)) = rx.try_recv() else {
            return;
        };
        if seq != self.fetch_seq {
            return;
        }
        self.loading = false;
        self.fetch_rx = None;
        match result {
            Ok(models) => {
                self.status = format!("已获取 {} 个模型", models.len());
                self.raw_models = models;
            }
            Err(err) => {
                self.status = format_status_line(&format!("获取模型失败: {err}"));
                self.raw_models.clear();
            }
        }
        self.rebuild_models();
    }

    fn rebuild_models(&mut self) {
        let filter = self.filter.to_ascii_lowercase();
        let mut grouped: BTreeMap<String, Vec<ModelEntry>> = BTreeMap::new();
        for model in &self.raw_models {
            if !filter.is_empty() && !model.to_ascii_lowercase().contains(&filter) {
                continue;
            }
            let org = model
                .split_once('/')
                .map(|(org, _)| org)
                .unwrap_or("All")
                .to_string();
            let name = model
                .split_once('/')
                .map(|(_, name)| name)
                .unwrap_or(model)
                .to_string();
            grouped
                .entry("All".to_string())
                .or_default()
                .push(ModelEntry::new(model, model));
            if org != "All" {
                grouped
                    .entry(org)
                    .or_default()
                    .push(ModelEntry::new(&name, model));
            }
        }
        self.orgs = grouped.keys().cloned().collect();
        if self.orgs.is_empty() {
            self.orgs.push("All".to_string());
        }
        self.org_idx = self.org_idx.min(self.orgs.len().saturating_sub(1));
        self.models = grouped.remove(&self.orgs[self.org_idx]).unwrap_or_default();
        self.model_idx = self.model_idx.min(self.models.len().saturating_sub(1));
    }

    fn add_provider(&mut self, stdout: &mut io::Stdout) -> Result<()> {
        if let Some(provider) = edit_provider_form(stdout, ProviderConfig::new_openai_compatible())?
        {
            self.config.upsert_provider(provider);
            self.provider_idx = self.config.providers.len().saturating_sub(1);
            self.refresh_models();
        }
        Ok(())
    }

    fn delete_provider(&mut self) {
        if self.config.providers.is_empty() {
            return;
        }
        let removed = self.config.providers.remove(self.provider_idx);
        if self.config.active_provider == removed.id {
            self.config.active_provider = self
                .config
                .providers
                .first()
                .map(|provider| provider.id.clone())
                .unwrap_or_default();
        }
        self.provider_idx = self
            .provider_idx
            .min(self.config.providers.len().saturating_sub(1));
        self.refresh_models();
    }

    fn select_or_edit(&mut self, stdout: &mut io::Stdout) -> Result<()> {
        match self.active_col {
            0 => {
                if let Some(provider) = self.config.providers.get(self.provider_idx).cloned() {
                    if let Some(provider) = edit_provider_form(stdout, provider)? {
                        let old_id = self.config.providers[self.provider_idx].id.clone();
                        self.config.providers[self.provider_idx] = provider.clone();
                        if self.config.active_provider == old_id {
                            self.config.active_provider = provider.id.clone();
                        }
                        self.refresh_models();
                    }
                }
            }
            2 => {
                if let (Some(provider), Some(model)) = (
                    self.config.providers.get_mut(self.provider_idx),
                    self.models.get(self.model_idx).cloned(),
                ) {
                    if edit_model_form(stdout, provider, &model.full)? {
                        self.config.active_provider = provider.id.clone();
                        self.status = format!("已更新模型设置: {}", model.full);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn toggle_model_activation(&mut self) {
        if self.active_col != 2 {
            return;
        }
        if let (Some(provider), Some(model)) = (
            self.config.providers.get_mut(self.provider_idx),
            self.models.get(self.model_idx),
        ) {
            if let Some(index) = provider.models.iter().position(|item| item == &model.full) {
                provider.models.remove(index);
                if provider.default_model == model.full {
                    provider.default_model = provider.models.first().cloned().unwrap_or_default();
                }
                self.status = format!("已取消激活模型: {}", model.full);
            } else {
                provider.models.push(model.full.clone());
                if provider.default_model.trim().is_empty() {
                    provider.default_model = model.full.clone();
                }
                self.status = format!("已激活模型: {}", model.full);
            }
        }
    }

    fn draw(&self, stdout: &mut io::Stdout) -> Result<()> {
        let (cols, rows) = terminal::size()?;
        let inner_x = 0;
        let inner_y = 0;
        let inner_w = cols;
        let inner_h = rows.saturating_sub(2);
        let left_w = inner_w.saturating_mul(28).saturating_div(100).max(20);
        let mid_w = inner_w.saturating_mul(22).saturating_div(100).max(16);
        let right_w = inner_w
            .saturating_sub(left_w)
            .saturating_sub(mid_w)
            .saturating_sub(2)
            .max(18);
        let providers = self
            .config
            .providers
            .iter()
            .map(|provider| {
                let active = if provider.id == self.config.active_provider {
                    "* "
                } else {
                    "  "
                };
                format!("{active}{}", provider.display_name)
            })
            .collect::<Vec<_>>();
        let models = self
            .models
            .iter()
            .map(|model| {
                let current = self
                    .config
                    .providers
                    .get(self.provider_idx)
                    .map(|provider| provider.default_model == model.full)
                    .unwrap_or(false);
                let active = self
                    .config
                    .providers
                    .get(self.provider_idx)
                    .map(|provider| provider.models.iter().any(|item| item == &model.full))
                    .unwrap_or(false);
                if current && active {
                    format!("{} [current active]", model.name)
                } else if current {
                    format!("{} [current]", model.name)
                } else if active {
                    format!("{} [active]", model.name)
                } else {
                    model.name.clone()
                }
            })
            .collect::<Vec<_>>();

        queue!(stdout, Clear(ClearType::All))?;
        draw_column(
            stdout,
            inner_x,
            inner_y,
            left_w,
            inner_h,
            " PROVIDERS ",
            &providers,
            self.provider_idx,
            self.active_col == 0,
        )?;
        draw_column(
            stdout,
            inner_x + left_w + 1,
            inner_y,
            mid_w,
            inner_h,
            " ORG ",
            &self.orgs,
            self.org_idx,
            self.active_col == 1,
        )?;
        let title = if self.filter.is_empty() {
            " MODELS ".to_string()
        } else {
            format!(" MODELS /{} ", self.filter)
        };
        draw_column(
            stdout,
            inner_x + left_w + mid_w + 2,
            inner_y,
            right_w,
            inner_h,
            &title,
            &models,
            self.model_idx,
            self.active_col == 2,
        )?;
        let help = if self.filter_mode {
            format!("搜索: {}_  [Enter]确认 [Esc]取消", self.filter)
        } else {
            "[h/l]切栏 [j/k]移动 [Tab]激活模型 [Enter]模型设置 [/]搜索 [r]刷新 [a]添加 [d]删除 [q]返回"
                .to_string()
        };
        let status = if self.loading {
            format!("{}", self.status)
        } else {
            self.status.clone()
        };
        queue!(
            stdout,
            MoveTo(0, rows.saturating_sub(2)),
            Clear(ClearType::CurrentLine),
            Print(truncate(&status, cols as usize))
        )?;
        queue!(
            stdout,
            MoveTo(0, rows.saturating_sub(1)),
            Clear(ClearType::CurrentLine),
            Print(truncate(&help, cols as usize))
        )?;
        stdout.flush()?;
        Ok(())
    }
}

type FetchResult = (u64, Result<Vec<String>, String>);

fn format_status_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Clone)]
struct ModelEntry {
    name: String,
    full: String,
}

impl ModelEntry {
    fn new(name: &str, full: &str) -> Self {
        Self {
            name: name.to_string(),
            full: full.to_string(),
        }
    }
}

fn fetch_models(provider: &ProviderConfig) -> Result<Vec<String>> {
    let api_key = provider.api_key.as_deref().unwrap_or_default();
    let mut api_key = if let Some(env_name) = api_key.strip_prefix("$env:") {
        std::env::var(env_name).unwrap_or_default()
    } else {
        api_key.to_string()
    };
    if api_key.is_empty() && provider.is_opencode_zen() {
        api_key = "public".to_string();
    }
    let url = models_url(&provider.base_url);
    let mut request = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(provider.timeout_seconds))
        .build()?
        .get(url)
        .header("Accept", "application/json")
        .header("User-Agent", "miyu-config");
    if !api_key.is_empty() {
        request = request.bearer_auth(api_key);
    }
    let response = request.send()?;
    let status = response.status();
    let body = response.text()?;
    if !status.is_success() {
        bail!("{status}: {body}");
    }
    let parsed: ModelsResponse = serde_json::from_str(&body)?;
    Ok(parsed
        .data
        .into_iter()
        .map(|model| model.id)
        .filter(|id| !id.is_empty())
        .collect())
}

fn models_url(base_url: &str) -> String {
    let mut url = base_url.trim().trim_end_matches('/').to_string();
    if url.ends_with("/chat/completions") {
        url.truncate(url.len() - "/chat/completions".len());
    }
    if url.ends_with("/v1") {
        format!("{url}/models")
    } else {
        format!("{url}/v1/models")
    }
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ModelInfo {
    id: String,
}

pub(crate) fn select_active_provider(
    stdout: &mut io::Stdout,
    config: &mut AppConfig,
) -> Result<()> {
    let choices = config.provider_model_choices();
    if choices.is_empty() {
        message(stdout, "没有可用 Provider，请先添加。")?;
        return Ok(());
    }
    let mut selected = choices
        .iter()
        .position(|choice| {
            config
                .provider(None)
                .map(|provider| {
                    provider.id == choice.provider_id && provider.default_model == choice.model
                })
                .unwrap_or(false)
        })
        .unwrap_or(0);
    loop {
        let options = choices
            .iter()
            .map(|choice| choice.label())
            .collect::<Vec<_>>();
        draw_menu(
            stdout,
            " SELECT PROVIDER/MODEL ",
            &options,
            selected,
            "[Enter]选择 [q]返回",
        )?;
        match read_key()? {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => selected = (selected + 1).min(options.len() - 1),
            KeyCode::Enter => {
                config.set_active_provider_model(
                    &choices[selected].provider_id,
                    &choices[selected].model,
                )?;
                return Ok(());
            }
            _ => {}
        }
    }
}

fn edit_provider_form(
    stdout: &mut io::Stdout,
    provider: ProviderConfig,
) -> Result<Option<ProviderConfig>> {
    let current_context_chars = provider
        .model_context_chars
        .get(&provider.default_model)
        .copied()
        .unwrap_or_default();
    let mut fields = vec![
        Field::new("配置 ID", provider.id.clone()),
        Field::new("显示名称", provider.display_name.clone()),
        Field::new("Base URL", provider.base_url.clone()),
        Field::new("协议", provider.protocol.clone()).choices(&[
            "auto",
            "openai-chat",
            "openai-responses",
            "anthropic",
        ]),
        Field::new(
            "API Key 或 $env:NAME",
            provider.api_key.clone().unwrap_or_default(),
        ),
        Field::new("当前模型", provider.default_model.clone()),
        Field::new("模型上下文字符数", current_context_chars.to_string()),
        Field::new("超时秒数", provider.timeout_seconds.to_string()),
        Field::new("Temperature", provider.temperature.to_string()),
        Field::new("思考等级", provider.thinking_level.clone())
            .choices(&["auto", "none", "low", "medium", "high", "xhigh", "max"]),
        Field::new("思考格式", provider.thinking_format.clone()).choices(&[
            "auto",
            "string",
            "object",
            "deepseek-thinking",
            "openai-chat-reasoning-effort",
            "reasoning",
            "anthropic-thinking",
            "disabled",
        ]),
        Field::textarea("自定义 Body JSON", provider.extra_body.clone()),
    ];
    if !run_form(stdout, " EDIT PROVIDER ", &mut fields)? {
        return Ok(None);
    }
    let default_model = fields[5].value.trim().to_string();
    let mut model_context_chars = provider.model_context_chars.clone();
    match fields[6].value.trim().parse::<usize>().unwrap_or_default() {
        0 => {
            model_context_chars.remove(&default_model);
        }
        value => {
            model_context_chars.insert(default_model.clone(), value);
        }
    }
    let mut models = provider.models.clone();
    if !default_model.trim().is_empty() && !models.iter().any(|item| item == &default_model) {
        models.push(default_model.clone());
    }
    let extra_body = normalize_extra_body(&fields[11].value)?;
    Ok(Some(ProviderConfig {
        id: fields[0].value.trim().to_string(),
        display_name: fields[1].value.trim().to_string(),
        base_url: normalize_base_url(&fields[2].value),
        protocol: fields[3].value.trim().to_string(),
        api_key: Some(fields[4].value.trim().to_string()).filter(|value| !value.is_empty()),
        models,
        model_context_chars,
        default_model,
        timeout_seconds: fields[7].value.trim().parse().unwrap_or(60),
        temperature: fields[8].value.trim().parse().unwrap_or(0.7),
        thinking_level: fields[9].value.trim().to_string(),
        thinking_format: fields[10].value.trim().to_string(),
        extra_body,
    }))
}

/// 规范化并校验自定义 Body JSON。
///
/// 参数:
/// - `value`: 表单中输入的 JSON 文本
///
/// 返回:
/// - 为空时返回空字符串，否则返回格式化后的 JSON 对象字符串
fn normalize_extra_body(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    let parsed = serde_json::from_str::<Value>(value)?;
    if !parsed.is_object() {
        bail!("自定义 Body JSON 必须是 JSON 对象");
    }
    Ok(serde_json::to_string_pretty(&parsed)?)
}

fn edit_model_form(
    stdout: &mut io::Stdout,
    provider: &mut ProviderConfig,
    model: &str,
) -> Result<bool> {
    let active = provider.models.iter().any(|item| item == model);
    let current = provider.default_model == model;
    let context_chars = provider
        .model_context_chars
        .get(model)
        .copied()
        .unwrap_or_default();
    let mut fields = vec![
        Field::boolean("激活模型", active),
        Field::boolean("设为当前模型", current),
        Field::new("模型上下文字符数", context_chars.to_string()),
    ];
    if !run_form(stdout, " EDIT MODEL ", &mut fields)? {
        return Ok(false);
    }
    let active = parse_bool_field(&fields[0].value)?;
    let current = parse_bool_field(&fields[1].value)?;
    if active {
        if !provider.models.iter().any(|item| item == model) {
            provider.models.push(model.to_string());
        }
    } else {
        provider.models.retain(|item| item != model);
    }
    if current || provider.default_model == model && !active {
        provider.default_model = if active {
            model.to_string()
        } else {
            provider.models.first().cloned().unwrap_or_default()
        };
        if !provider.default_model.is_empty()
            && !provider
                .models
                .iter()
                .any(|item| item == &provider.default_model)
        {
            provider.models.push(provider.default_model.clone());
        }
    }
    match fields[2].value.trim().parse::<usize>().unwrap_or_default() {
        0 => {
            provider.model_context_chars.remove(model);
        }
        value => {
            provider
                .model_context_chars
                .insert(model.to_string(), value);
        }
    }
    Ok(true)
}

fn normalize_base_url(value: &str) -> String {
    let mut url = value.trim().trim_end_matches('/').to_string();
    if url.ends_with("/chat/completions") {
        url.truncate(url.len() - "/chat/completions".len());
    }
    url
}
