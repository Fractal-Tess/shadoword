use crate::error::{ApiError, ApiResult};
use shadoword_core::remote_contracts::{DownloadJobState, DownloadJobStatus};
use shadoword_core::{
    download_whisper_model_with_progress, resolve_whisper_model, unknown_model_error,
    ModelDownloadProgress,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct DownloadJobs {
    inner: Arc<DownloadJobsInner>,
}

#[derive(Default)]
struct DownloadJobsInner {
    next_id: AtomicU64,
    jobs: Mutex<HashMap<String, DownloadJobStatus>>,
}

impl DownloadJobs {
    pub fn start(&self, model_id: String, target_dir: PathBuf) -> ApiResult<DownloadJobStatus> {
        let spec = resolve_whisper_model(&model_id)
            .ok_or_else(|| ApiError::bad_request(unknown_model_error(&model_id).to_string()))?;
        let mut jobs = self.inner.jobs.lock().expect("download jobs lock poisoned");
        if let Some(existing) = jobs.values().find(|status| {
            status.model_id == spec.id
                && matches!(
                    status.state,
                    DownloadJobState::Queued | DownloadJobState::Running
                )
        }) {
            return Ok(existing.clone());
        }

        let id = self
            .inner
            .next_id
            .fetch_add(1, Ordering::Relaxed)
            .to_string();
        let status = DownloadJobStatus {
            id: id.clone(),
            model_id: spec.id.to_string(),
            state: DownloadJobState::Queued,
            downloaded: 0,
            total: spec.size_bytes,
            path: None,
            skipped: false,
            verified: false,
            error: None,
        };
        if jobs.len() >= 256 {
            let oldest_finished = jobs
                .iter()
                .filter(|(_, job)| {
                    matches!(
                        job.state,
                        DownloadJobState::Succeeded | DownloadJobState::Failed
                    )
                })
                .min_by_key(|(id, _)| id.parse::<u64>().unwrap_or(u64::MAX))
                .map(|(id, _)| id.clone());
            if let Some(oldest_finished) = oldest_finished {
                jobs.remove(&oldest_finished);
            }
        }
        jobs.insert(status.id.clone(), status.clone());
        drop(jobs);

        let jobs = self.clone();
        let job_id = id.clone();
        tokio::task::spawn_blocking(move || {
            jobs.update(&job_id, |status| {
                status.state = DownloadJobState::Running;
            });

            let result = download_whisper_model_with_progress(spec, &target_dir, |progress| {
                jobs.record_progress(&job_id, progress);
            });

            match result {
                Ok(result) => jobs.update(&job_id, |status| {
                    status.state = DownloadJobState::Succeeded;
                    status.path = Some(result.path.to_string_lossy().into_owned());
                    status.skipped = result.skipped;
                    status.verified = result.verified;
                    status.downloaded = status.total;
                }),
                Err(error) => jobs.update(&job_id, |status| {
                    status.state = DownloadJobState::Failed;
                    status.error = Some(error.to_string());
                }),
            }
        });

        Ok(status)
    }

    pub fn is_active(&self, model_id: &str) -> bool {
        self.inner
            .jobs
            .lock()
            .expect("download jobs lock poisoned")
            .values()
            .any(|status| {
                status.model_id == model_id
                    && matches!(
                        status.state,
                        DownloadJobState::Queued | DownloadJobState::Running
                    )
            })
    }

    pub fn get(&self, id: &str) -> ApiResult<DownloadJobStatus> {
        self.inner
            .jobs
            .lock()
            .expect("download jobs lock poisoned")
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::not_found("download job not found"))
    }

    fn record_progress(&self, id: &str, progress: ModelDownloadProgress) {
        self.update(id, |status| {
            status.downloaded = progress.downloaded;
            status.total = progress.total;
        });
    }

    fn update(&self, id: &str, update: impl FnOnce(&mut DownloadJobStatus)) {
        if let Some(status) = self
            .inner
            .jobs
            .lock()
            .expect("download jobs lock poisoned")
            .get_mut(id)
        {
            update(status);
        }
    }
}
