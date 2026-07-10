use super::assembler::EventAssembler;
use super::model_override::resolve_run_config;
use super::{EventJournal, WebEvent};
use crate::agent::AgentMode;
use crate::paths::MiyuPaths;
use crate::runner::{RunnerSubmission, SessionRunner, SubmissionSource, UserInputSubmission};
use crate::web::workspaces::WorkspaceInfo;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::AbortHandle;

const RUN_JOURNAL_CAPACITY: usize = 32;

/// 启动一轮 Web 对话所需参数。
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct StartRunRequest {
    pub session_id: String,
    pub input: String,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub image_urls: Vec<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub thinking_level: Option<String>,
}

/// 活动运行摘要。
#[derive(Clone, Debug, Serialize)]
pub(crate) struct ActiveRunInfo {
    pub run_id: String,
    pub workspace_id: String,
    pub session_id: String,
}

struct ActiveRun {
    info: ActiveRunInfo,
    abort: AbortHandle,
}

#[derive(Default)]
struct RunJournals {
    entries: HashMap<String, EventJournal>,
    order: VecDeque<String>,
}

/// 管理 Web 运行互斥、事件日志和中断句柄。
#[derive(Clone)]
pub(crate) struct RunManager {
    active: Arc<Mutex<Option<ActiveRun>>>,
    journals: Arc<RwLock<RunJournals>>,
}

impl RunManager {
    /// 创建空运行管理器。
    pub(crate) fn new() -> Self {
        Self {
            active: Arc::new(Mutex::new(None)),
            journals: Arc::new(RwLock::new(RunJournals::default())),
        }
    }

    /// 启动一轮 Agent 运行。
    ///
    /// 参数:
    /// - `paths`: Miyu 路径集合
    /// - `workspace`: 当前活动工作区
    /// - `request`: 用户输入
    ///
    /// 返回:
    /// - 活动运行摘要
    pub(crate) async fn start(
        &self,
        paths: MiyuPaths,
        workspace: WorkspaceInfo,
        request: StartRunRequest,
    ) -> Result<ActiveRunInfo> {
        if request.input.trim().is_empty()
            && request.image_url.is_none()
            && request.image_urls.is_empty()
        {
            bail!("message cannot be empty");
        }
        parse_mode(request.mode.as_deref())?;
        let mut active = self.active.lock().await;
        if let Some(active) = active.as_ref() {
            bail!("run {} is already active", active.info.run_id);
        }
        let run_id = format!("run_{}", uuid::Uuid::new_v4().simple());
        let info = ActiveRunInfo {
            run_id: run_id.clone(),
            workspace_id: workspace.id.clone(),
            session_id: request.session_id.clone(),
        };
        let journal = EventJournal::new();
        self.insert_journal(run_id.clone(), journal.clone()).await;
        let (start_tx, start_rx) = oneshot::channel();
        let manager = self.clone();
        let task_info = info.clone();
        let handle = tokio::spawn(async move {
            let _ = start_rx.await;
            run_agent(paths, request, task_info.clone(), journal.clone()).await;
            manager.clear_active_if(&task_info.run_id).await;
        });
        *active = Some(ActiveRun {
            info: info.clone(),
            abort: handle.abort_handle(),
        });
        drop(active);
        let _ = start_tx.send(());
        Ok(info)
    }

    /// 返回当前活动运行。
    pub(crate) async fn active(&self) -> Option<ActiveRunInfo> {
        self.active
            .lock()
            .await
            .as_ref()
            .map(|active| active.info.clone())
    }

    /// 判断是否存在活动运行。
    pub(crate) async fn is_active(&self) -> bool {
        self.active.lock().await.is_some()
    }

    /// 中断指定运行。
    ///
    /// 参数:
    /// - `run_id`: 运行 ID
    ///
    /// 返回:
    /// - 是否执行了中断
    pub(crate) async fn stop(&self, run_id: &str) -> Result<bool> {
        let mut active = self.active.lock().await;
        let Some(current) = active.as_ref() else {
            return Ok(false);
        };
        if current.info.run_id != run_id {
            bail!("run is not active: {run_id}");
        }
        current.abort.abort();
        let info = current.info.clone();
        *active = None;
        drop(active);
        if let Some(journal) = self.journal(run_id).await {
            journal.publish(WebEvent::new(
                &info.run_id,
                &info.workspace_id,
                &info.session_id,
                "run.interrupted",
                json!({}),
            ));
        }
        Ok(true)
    }

    /// 返回指定运行事件日志。
    pub(crate) async fn journal(&self, run_id: &str) -> Option<EventJournal> {
        self.journals.read().await.entries.get(run_id).cloned()
    }

    /// 保存运行事件日志并移除最早的过期日志。
    ///
    /// 参数:
    /// - `run_id`: 运行 ID
    /// - `journal`: 运行事件日志
    async fn insert_journal(&self, run_id: String, journal: EventJournal) {
        let mut journals = self.journals.write().await;
        journals.entries.insert(run_id.clone(), journal);
        journals.order.push_back(run_id);
        while journals.order.len() > RUN_JOURNAL_CAPACITY {
            if let Some(expired_id) = journals.order.pop_front() {
                journals.entries.remove(&expired_id);
            }
        }
    }

    /// 清理指定活动运行。
    async fn clear_active_if(&self, run_id: &str) {
        let mut active = self.active.lock().await;
        if active
            .as_ref()
            .is_some_and(|active| active.info.run_id == run_id)
        {
            *active = None;
        }
    }
}

/// 执行 Agent 并把 RunnerEvent 写入事件日志。
async fn run_agent(
    paths: MiyuPaths,
    request: StartRunRequest,
    info: ActiveRunInfo,
    journal: EventJournal,
) {
    let mode = match parse_mode(request.mode.as_deref()) {
        Ok(mode) => mode,
        Err(error) => {
            journal.publish(WebEvent::new(
                &info.run_id,
                &info.workspace_id,
                &info.session_id,
                "run.failed",
                json!({ "message": error.to_string() }),
            ));
            return;
        }
    };
    let mut input = UserInputSubmission::new(request.input, mode);
    input = input.with_image_urls(request.image_url.into_iter().chain(request.image_urls));
    let submission = RunnerSubmission::user_input(SubmissionSource::Web, input)
        .with_session_id(info.session_id.clone())
        .with_final_summary(true);
    let mut assembler = EventAssembler::new(&info.run_id, &info.workspace_id, &info.session_id);
    let mut sink = |event| {
        for event in assembler.map(event) {
            journal.publish(event);
        }
        Ok(())
    };
    let run_config = match resolve_run_config(
        &paths,
        request.provider_id.as_deref(),
        request.model.as_deref(),
        request.thinking_level.as_deref(),
    ) {
        Ok(config) => config,
        Err(error) => {
            journal.publish(WebEvent::new(
                &info.run_id,
                &info.workspace_id,
                &info.session_id,
                "run.failed",
                json!({ "message": error.to_string() }),
            ));
            return;
        }
    };
    let runner = match run_config {
        Some(config) => SessionRunner::new(&paths).with_config(config),
        None => SessionRunner::new(&paths),
    };
    if let Err(error) = runner.run_submission(submission, &mut sink).await {
        journal.publish(WebEvent::new(
            &info.run_id,
            &info.workspace_id,
            &info.session_id,
            "run.failed",
            json!({ "message": error.to_string() }),
        ));
    }
}

/// 解析 Web 端运行模式。
fn parse_mode(value: Option<&str>) -> Result<AgentMode> {
    match value.unwrap_or("yolo").trim().to_ascii_lowercase().as_str() {
        "plan" => Ok(AgentMode::Plan),
        "yolo" | "" => Ok(AgentMode::Yolo),
        value => bail!("unsupported run mode: {value}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn keeps_only_recent_run_journals() {
        let manager = RunManager::new();
        for index in 0..=RUN_JOURNAL_CAPACITY {
            manager
                .insert_journal(format!("run-{index}"), EventJournal::new())
                .await;
        }
        assert!(manager.journal("run-0").await.is_none());
        assert!(manager
            .journal(&format!("run-{RUN_JOURNAL_CAPACITY}"))
            .await
            .is_some());
    }
}
