use super::repository::CronRepository;
use crate::agent::AgentMode;
use crate::paths::MiyuPaths;
use crate::runner::{RunnerSubmission, SessionRunner, SubmissionSource, UserInputSubmission};
use anyhow::Result;
use chrono::Utc;
use std::time::Duration;

/// 运行 Gateway 专属定时任务调度循环。
pub(crate) async fn run_scheduler(paths: MiyuPaths) -> Result<()> {
    let repository = CronRepository::new(&paths)?;
    loop {
        if let Some(job) = repository.next_due(Utc::now().timestamp())? {
            let input = UserInputSubmission::new(job.prompt.clone(), AgentMode::Yolo);
            let submission = RunnerSubmission::user_input(SubmissionSource::Gateway, input)
                .with_session_id(job.session_id.clone());
            let mut sink = |_| Ok(());
            match SessionRunner::new(&paths)
                .run_submission(submission, &mut sink)
                .await
            {
                Ok(_) => repository.complete(&job, Utc::now().timestamp())?,
                Err(error) => repository.fail(&job, &error.to_string(), Utc::now().timestamp())?,
            }
            continue;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
