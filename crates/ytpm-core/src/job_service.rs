//! In-memory background media export queue.
//!
//! The queue deliberately has no database persistence: `project.json` and
//! portable timeline files remain authoritative. Queue records exist only for
//! the lifetime of this process, so unfinished jobs must be enqueued again
//! after an application restart.

use crate::{
    media_service::export_timeline_controlled, render_manifest,
    timeline_service::validate_output_relative_path, MediaExportResult, MediaJobStatus, Result,
    Timeline, YtpmError,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex, MutexGuard,
};
use std::thread;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaJobKind {
    Export,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaQueueJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl MediaQueueJobStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaJobRecord {
    pub id: String,
    pub project_path: String,
    pub kind: MediaJobKind,
    pub status: MediaQueueJobStatus,
    pub progress: u8,
    pub output_relative_path: String,
    pub message: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

struct ExportJob {
    id: String,
    project_dir: PathBuf,
    timeline: Timeline,
    output_relative_path: String,
}

struct SharedJob {
    record: MediaJobRecord,
    project_key: String,
    cancel: Arc<AtomicBool>,
}

trait ExportRunner: Send + Sync + 'static {
    fn run(
        &self,
        project_dir: &Path,
        timeline: &Timeline,
        output_relative_path: &str,
        operation_id: &str,
        cancel: &AtomicBool,
        progress: &dyn Fn(u8),
    ) -> Result<MediaExportResult>;
}

struct ControlledExportRunner;

impl ExportRunner for ControlledExportRunner {
    fn run(
        &self,
        project_dir: &Path,
        timeline: &Timeline,
        output_relative_path: &str,
        operation_id: &str,
        cancel: &AtomicBool,
        progress: &dyn Fn(u8),
    ) -> Result<MediaExportResult> {
        export_timeline_controlled(
            project_dir,
            timeline,
            output_relative_path,
            operation_id,
            cancel,
            progress,
        )
    }
}

/// A cloneable handle to one process-local, single-worker media queue.
///
/// Clones share job state and the same worker. Constructing a separate queue
/// creates a separate worker, which is useful for isolated application states
/// and tests.
#[derive(Clone)]
pub struct MediaJobQueue {
    sender: mpsc::Sender<ExportJob>,
    jobs: Arc<Mutex<HashMap<String, SharedJob>>>,
}

impl Default for MediaJobQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaJobQueue {
    pub fn new() -> Self {
        Self::with_runner(Arc::new(ControlledExportRunner))
    }

    fn with_runner(runner: Arc<dyn ExportRunner>) -> Self {
        let (sender, receiver) = mpsc::channel::<ExportJob>();
        let jobs = Arc::new(Mutex::new(HashMap::new()));
        let worker_jobs = Arc::clone(&jobs);
        thread::Builder::new()
            .name("ytpm-media-export-worker".into())
            .spawn(move || worker_loop(receiver, worker_jobs, runner))
            .expect("failed to start ytpm media export worker");
        Self { sender, jobs }
    }

    pub fn enqueue_export(
        &self,
        project_dir: PathBuf,
        timeline: Timeline,
        output_relative_path: String,
    ) -> Result<MediaJobRecord> {
        // Reject invalid timelines before they enter the worker so callers get
        // immediate, actionable feedback instead of a delayed queue failure.
        validate_output_relative_path(&output_relative_path)?;
        render_manifest(&timeline)?;
        let (project_path, project_key) = normalize_project_scope(&project_dir)?;

        let id = Uuid::new_v4().to_string();
        let record = MediaJobRecord {
            id: id.clone(),
            project_path,
            kind: MediaJobKind::Export,
            status: MediaQueueJobStatus::Queued,
            progress: 0,
            output_relative_path: output_relative_path.clone(),
            message: Some("匯出工作已加入背景佇列。".into()),
            created_at: now(),
            started_at: None,
            finished_at: None,
        };
        lock_jobs(&self.jobs).insert(
            id.clone(),
            SharedJob {
                record: record.clone(),
                project_key,
                cancel: Arc::new(AtomicBool::new(false)),
            },
        );

        if self
            .sender
            .send(ExportJob {
                id: id.clone(),
                project_dir,
                timeline,
                output_relative_path,
            })
            .is_err()
        {
            lock_jobs(&self.jobs).remove(&id);
            return Err(YtpmError::InvalidInput(
                "背景媒體 worker 已停止，請重新啟動應用程式後再加入工作。".into(),
            ));
        }
        Ok(record)
    }

    pub fn status(&self, id: &str) -> Result<MediaJobRecord> {
        lock_jobs(&self.jobs)
            .get(id)
            .map(|job| job.record.clone())
            .ok_or_else(|| unknown_job(id))
    }

    pub fn status_for_project(&self, project_dir: &Path, id: &str) -> Result<MediaJobRecord> {
        let (_, project_key) = normalize_project_scope(project_dir)?;
        lock_jobs(&self.jobs)
            .get(id)
            .filter(|job| job.project_key == project_key)
            .map(|job| job.record.clone())
            .ok_or_else(|| unknown_job(id))
    }

    pub fn list(&self) -> Vec<MediaJobRecord> {
        sorted_records(lock_jobs(&self.jobs).values().map(|job| job.record.clone()))
    }

    pub fn list_for_project(&self, project_dir: &Path) -> Result<Vec<MediaJobRecord>> {
        let (_, project_key) = normalize_project_scope(project_dir)?;
        let records = lock_jobs(&self.jobs)
            .values()
            .filter(|job| job.project_key == project_key)
            .map(|job| job.record.clone())
            .collect::<Vec<_>>();
        Ok(sorted_records(records))
    }

    pub fn cancel(&self, id: &str) -> Result<MediaJobRecord> {
        let mut jobs = lock_jobs(&self.jobs);
        let job = jobs.get_mut(id).ok_or_else(|| unknown_job(id))?;
        cancel_job(job)
    }

    pub fn cancel_for_project(&self, project_dir: &Path, id: &str) -> Result<MediaJobRecord> {
        let (_, project_key) = normalize_project_scope(project_dir)?;
        let mut jobs = lock_jobs(&self.jobs);
        let job = jobs
            .get_mut(id)
            .filter(|job| job.project_key == project_key)
            .ok_or_else(|| unknown_job(id))?;
        cancel_job(job)
    }
}

fn cancel_job(job: &mut SharedJob) -> Result<MediaJobRecord> {
    if job.record.status.is_terminal() {
        return Ok(job.record.clone());
    }

    job.cancel.store(true, Ordering::Release);
    match job.record.status {
        MediaQueueJobStatus::Queued => {
            job.record.status = MediaQueueJobStatus::Cancelled;
            job.record.message = Some("工作在開始前已取消。".into());
            job.record.finished_at = Some(now());
        }
        MediaQueueJobStatus::Running => {
            job.record.message = Some("正在取消 FFmpeg 匯出工作…".into());
        }
        _ => {}
    }
    Ok(job.record.clone())
}

fn worker_loop(
    receiver: mpsc::Receiver<ExportJob>,
    jobs: Arc<Mutex<HashMap<String, SharedJob>>>,
    runner: Arc<dyn ExportRunner>,
) {
    while let Ok(job) = receiver.recv() {
        let cancel = {
            let mut records = lock_jobs(&jobs);
            let Some(shared) = records.get_mut(&job.id) else {
                continue;
            };
            if shared.cancel.load(Ordering::Acquire)
                || shared.record.status == MediaQueueJobStatus::Cancelled
            {
                if shared.record.finished_at.is_none() {
                    shared.record.status = MediaQueueJobStatus::Cancelled;
                    shared.record.message = Some("工作在開始前已取消。".into());
                    shared.record.finished_at = Some(now());
                }
                continue;
            }
            shared.record.status = MediaQueueJobStatus::Running;
            shared.record.progress = 1;
            shared.record.message = Some("FFmpeg 背景匯出進行中。".into());
            shared.record.started_at = Some(now());
            Arc::clone(&shared.cancel)
        };

        let progress_jobs = Arc::clone(&jobs);
        let progress_id = job.id.clone();
        let progress = move |value: u8| {
            let mut records = lock_jobs(&progress_jobs);
            if let Some(shared) = records.get_mut(&progress_id) {
                if shared.record.status == MediaQueueJobStatus::Running {
                    shared.record.progress = value.clamp(1, 99);
                }
            }
        };
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runner.run(
                &job.project_dir,
                &job.timeline,
                &job.output_relative_path,
                &job.id,
                &cancel,
                &progress,
            )
        }));

        let mut records = lock_jobs(&jobs);
        let Some(shared) = records.get_mut(&job.id) else {
            continue;
        };
        shared.record.finished_at = Some(now());
        match outcome {
            Ok(Ok(result)) if result.status == MediaJobStatus::Completed => {
                shared.record.status = MediaQueueJobStatus::Completed;
                shared.record.progress = 100;
                shared.record.message = result.message.or_else(|| Some("背景匯出完成。".into()));
                if let Some(path) = result.output_relative_path {
                    shared.record.output_relative_path = path;
                }
            }
            Ok(Ok(result))
                if cancel.load(Ordering::Acquire) || result.status == MediaJobStatus::Cancelled =>
            {
                shared.record.status = MediaQueueJobStatus::Cancelled;
                shared.record.message = result.message.or_else(|| Some("背景匯出已取消。".into()));
            }
            Ok(Ok(result)) => {
                shared.record.status = MediaQueueJobStatus::Failed;
                shared.record.message = result.message.or_else(|| Some("背景匯出失敗。".into()));
            }
            Ok(Err(error)) if cancel.load(Ordering::Acquire) => {
                shared.record.status = MediaQueueJobStatus::Cancelled;
                shared.record.message = Some(format!("背景匯出已取消：{error}"));
            }
            Ok(Err(error)) => {
                shared.record.status = MediaQueueJobStatus::Failed;
                shared.record.message = Some(error.to_string());
            }
            Err(_) => {
                shared.record.status = MediaQueueJobStatus::Failed;
                shared.record.message =
                    Some("背景媒體 worker 發生未預期錯誤，工作已安全停止。".into());
            }
        }
    }
}

fn lock_jobs(
    jobs: &Mutex<HashMap<String, SharedJob>>,
) -> MutexGuard<'_, HashMap<String, SharedJob>> {
    jobs.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn sorted_records(records: impl IntoIterator<Item = MediaJobRecord>) -> Vec<MediaJobRecord> {
    let mut records = records.into_iter().collect::<Vec<_>>();
    records.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    records
}

fn normalize_project_scope(project_dir: &Path) -> Result<(String, String)> {
    if project_dir.as_os_str().is_empty()
        || project_dir
            .components()
            .any(|component| component == Component::ParentDir)
    {
        return Err(YtpmError::InvalidInput(
            "project path 不可為空或包含 ..".into(),
        ));
    }
    let absolute = if project_dir.is_absolute() {
        project_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| YtpmError::io(project_dir, error))?
            .join(project_dir)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        if component != Component::CurDir {
            normalized.push(component.as_os_str());
        }
    }
    let project_path = normalized.to_string_lossy().into_owned();
    #[cfg(windows)]
    let project_key = project_path.replace('/', "\\").to_lowercase();
    #[cfg(not(windows))]
    let project_key = project_path.clone();
    Ok((project_path, project_key))
}

fn unknown_job(id: &str) -> YtpmError {
    YtpmError::InvalidInput(format!(
        "找不到背景媒體工作 {id}。工作狀態只保存在記憶體中；應用程式重啟後請重新加入工作。"
    ))
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TimelineClip;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::time::{Duration, Instant};

    #[derive(Clone)]
    enum RunnerBehavior {
        Complete,
        CompleteAfterCancel,
        Fail,
        WaitForCancel,
        Block(Arc<AtomicBool>),
    }

    struct FakeRunner {
        behavior: RunnerBehavior,
        calls: Arc<AtomicUsize>,
    }

    impl ExportRunner for FakeRunner {
        fn run(
            &self,
            _project_dir: &Path,
            _timeline: &Timeline,
            output_relative_path: &str,
            operation_id: &str,
            cancel: &AtomicBool,
            progress: &dyn Fn(u8),
        ) -> Result<MediaExportResult> {
            self.calls.fetch_add(1, AtomicOrdering::SeqCst);
            progress(35);
            match &self.behavior {
                RunnerBehavior::Complete => Ok(completed(operation_id, output_relative_path)),
                RunnerBehavior::CompleteAfterCancel => {
                    wait_until(|| cancel.load(Ordering::Acquire));
                    Ok(completed(operation_id, output_relative_path))
                }
                RunnerBehavior::Fail => Err(YtpmError::InvalidInput("測試匯出失敗".into())),
                RunnerBehavior::WaitForCancel => {
                    wait_until(|| cancel.load(Ordering::Acquire));
                    Ok(cancelled(operation_id))
                }
                RunnerBehavior::Block(release) => {
                    wait_until(|| release.load(Ordering::Acquire));
                    Ok(completed(operation_id, output_relative_path))
                }
            }
        }
    }

    #[test]
    fn completes_job_and_reports_progress() {
        let (queue, calls) = queue_with(RunnerBehavior::Complete);
        let queued = queue
            .enqueue_export(
                PathBuf::from("project"),
                timeline(),
                "exports/out.mp4".into(),
            )
            .unwrap();
        let completed = wait_for_terminal(&queue, &queued.id);

        assert_eq!(completed.status, MediaQueueJobStatus::Completed);
        assert_eq!(completed.progress, 100);
        assert_eq!(completed.output_relative_path, "exports/out.mp4");
        assert!(completed.project_path.ends_with("project"));
        assert!(completed.started_at.is_some());
        assert!(completed.finished_at.is_some());
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(queue.list(), vec![completed]);
    }

    #[test]
    fn records_export_failure_without_stopping_worker() {
        let (queue, calls) = queue_with(RunnerBehavior::Fail);
        let first = queue
            .enqueue_export(
                PathBuf::from("project"),
                timeline(),
                "exports/one.mp4".into(),
            )
            .unwrap();
        let second = queue
            .enqueue_export(
                PathBuf::from("project"),
                timeline(),
                "exports/two.mp4".into(),
            )
            .unwrap();

        let failed_first = wait_for_terminal(&queue, &first.id);
        let failed_second = wait_for_terminal(&queue, &second.id);
        assert_eq!(failed_first.status, MediaQueueJobStatus::Failed);
        assert_eq!(failed_second.status, MediaQueueJobStatus::Failed);
        assert!(failed_first.message.unwrap().contains("測試匯出失敗"));
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 2);
    }

    #[test]
    fn cancels_running_job_through_shared_atomic_flag() {
        let (queue, calls) = queue_with(RunnerBehavior::WaitForCancel);
        let job = queue
            .enqueue_export(
                PathBuf::from("project"),
                timeline(),
                "exports/out.mp4".into(),
            )
            .unwrap();
        wait_until(|| queue.status(&job.id).unwrap().status == MediaQueueJobStatus::Running);

        let cancelling = queue.cancel(&job.id).unwrap();
        assert_eq!(cancelling.status, MediaQueueJobStatus::Running);
        let cancelled = wait_for_terminal(&queue, &job.id);
        assert_eq!(cancelled.status, MediaQueueJobStatus::Cancelled);
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 1);
    }

    #[test]
    fn completed_export_wins_over_late_cancel_flag() {
        let (queue, _) = queue_with(RunnerBehavior::CompleteAfterCancel);
        let job = queue
            .enqueue_export(
                PathBuf::from("project"),
                timeline(),
                "exports/out.mp4".into(),
            )
            .unwrap();
        wait_until(|| queue.status(&job.id).unwrap().status == MediaQueueJobStatus::Running);

        queue.cancel(&job.id).unwrap();
        let completed = wait_for_terminal(&queue, &job.id);
        assert_eq!(completed.status, MediaQueueJobStatus::Completed);
        assert_eq!(completed.progress, 100);
    }

    #[test]
    fn cancelled_queued_job_never_reaches_exporter() {
        let release = Arc::new(AtomicBool::new(false));
        let (queue, calls) = queue_with(RunnerBehavior::Block(Arc::clone(&release)));
        let running = queue
            .enqueue_export(
                PathBuf::from("project"),
                timeline(),
                "exports/one.mp4".into(),
            )
            .unwrap();
        wait_until(|| queue.status(&running.id).unwrap().status == MediaQueueJobStatus::Running);
        let queued = queue
            .enqueue_export(
                PathBuf::from("project"),
                timeline(),
                "exports/two.mp4".into(),
            )
            .unwrap();

        let cancelled = queue.cancel(&queued.id).unwrap();
        assert_eq!(cancelled.status, MediaQueueJobStatus::Cancelled);
        release.store(true, Ordering::Release);
        assert_eq!(
            wait_for_terminal(&queue, &running.id).status,
            MediaQueueJobStatus::Completed
        );
        wait_until(|| queue.status(&queued.id).unwrap().status.is_terminal());
        assert_eq!(calls.load(AtomicOrdering::SeqCst), 1);
    }

    #[test]
    fn unknown_job_explains_process_local_storage() {
        let (queue, _) = queue_with(RunnerBehavior::Complete);
        let error = queue.status("missing").unwrap_err().to_string();
        assert!(error.contains("只保存在記憶體"));
    }

    #[test]
    fn project_scoped_apis_do_not_expose_other_projects_jobs() {
        let (queue, _) = queue_with(RunnerBehavior::Complete);
        let first_project = PathBuf::from("project-one");
        let second_project = PathBuf::from("project-two");
        let first = queue
            .enqueue_export(first_project.clone(), timeline(), "exports/one.mp4".into())
            .unwrap();
        let second = queue
            .enqueue_export(second_project.clone(), timeline(), "exports/two.mp4".into())
            .unwrap();
        wait_for_terminal(&queue, &first.id);
        wait_for_terminal(&queue, &second.id);

        assert_eq!(queue.list_for_project(&first_project).unwrap().len(), 1);
        assert_eq!(
            queue
                .status_for_project(&first_project, &first.id)
                .unwrap()
                .id,
            first.id
        );
        assert!(queue
            .status_for_project(&first_project, &second.id)
            .is_err());
        assert!(queue
            .cancel_for_project(&first_project, &second.id)
            .is_err());
    }

    #[test]
    fn queue_rejects_non_mp4_and_windows_unsafe_outputs_before_insertion() {
        let (queue, _) = queue_with(RunnerBehavior::Complete);
        for output in [
            "09_exports/final.webm",
            "09_exports/CON.mp4",
            "09_exports/final:stream.mp4",
            "09_exports/final.mp4 ",
        ] {
            assert!(queue
                .enqueue_export(PathBuf::from("project"), timeline(), output.into())
                .is_err());
        }
        assert!(queue.list().is_empty());
    }

    fn queue_with(behavior: RunnerBehavior) -> (MediaJobQueue, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let queue = MediaJobQueue::with_runner(Arc::new(FakeRunner {
            behavior,
            calls: Arc::clone(&calls),
        }));
        (queue, calls)
    }

    fn wait_for_terminal(queue: &MediaJobQueue, id: &str) -> MediaJobRecord {
        wait_until(|| queue.status(id).unwrap().status.is_terminal());
        queue.status(id).unwrap()
    }

    fn wait_until(mut predicate: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !predicate() {
            assert!(Instant::now() < deadline, "timed out waiting for media job");
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn completed(operation_id: &str, output_relative_path: &str) -> MediaExportResult {
        MediaExportResult {
            operation_id: operation_id.into(),
            status: MediaJobStatus::Completed,
            progress: 100,
            output_relative_path: Some(output_relative_path.into()),
            message: Some("完成".into()),
        }
    }

    fn cancelled(operation_id: &str) -> MediaExportResult {
        MediaExportResult {
            operation_id: operation_id.into(),
            status: MediaJobStatus::Cancelled,
            progress: 0,
            output_relative_path: None,
            message: Some("取消".into()),
        }
    }

    fn timeline() -> Timeline {
        let mut timeline = Timeline {
            duration_ms: 1_000,
            ..Timeline::default()
        };
        timeline.tracks[0].clips.push(TimelineClip {
            id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".into(),
            asset_id: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb".into(),
            relative_path: "assets/video/example.mp4".into(),
            label: "Example".into(),
            start_ms: 0,
            in_ms: 0,
            out_ms: 1_000,
            duration_ms: 1_000,
            volume: 1.0,
            muted: false,
            transition: None,
            effects: Vec::new(),
        });
        timeline
    }
}
