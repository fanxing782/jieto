use crate::task::ScheduledTask;
use anyhow::Result;
use std::any::Any;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};

pub struct TaskScheduler {
    scheduler: Mutex<Option<JobScheduler>>,
    app_state: Arc<dyn Any + Send + Sync>,
    task_count: AtomicUsize,
    is_started: AtomicBool,
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self {
            scheduler: Mutex::new(None),
            app_state: Arc::new(()),
            task_count: AtomicUsize::new(0),
            is_started: AtomicBool::new(false),
        }
    }
}

impl fmt::Debug for TaskScheduler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskScheduler")
            .field("task_count", &self.task_count.load(Ordering::Relaxed))
            .finish()
    }
}

impl TaskScheduler {
    pub async fn new(app_state: Arc<dyn Any + Send + Sync>) -> Result<Self> {
        let job_scheduler = JobScheduler::new().await?;

        Ok(Self {
            scheduler: Mutex::new(Some(job_scheduler)),
            app_state,
            task_count: AtomicUsize::new(0),
            is_started: AtomicBool::new(false),
        })
    }

    pub async fn register_task(&self, task: Box<dyn ScheduledTask>) -> Result<()> {
        let mut guard = self.scheduler.lock().await;

        let scheduler = guard.as_mut().ok_or_else(|| {
            anyhow::anyhow!("[job] job scheduler not initialized or already shutdown")
        })?;

        let app_state = Arc::clone(&self.app_state);
        let cron_expr = task.cron_expression().to_string();
        let task_name = task.task_name().to_string();

        log::info!(
            "[job] registering task: {} with cron: {}",
            task_name,
            cron_expr
        );

        let task = Arc::new(task);

        let job = Job::new_async(cron_expr.as_str(), move |_uuid, _lock| {
            let task = Arc::clone(&task);
            let state = Arc::clone(&app_state);
            Box::pin(async move {
                log::debug!("️[job] [{}] starting execution...", task.task_name());
                task.execute(state).await;
                log::debug!("[job] [{}] completed execution", task.task_name());
            })
        })?;

        scheduler.add(job).await?;

        self.task_count.fetch_add(1, Ordering::SeqCst);

        log::info!("[job] successfully registered task: {}", task_name);
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        let count = self.get_task_count();
        if count == 0 {
            log::info!("[job] The scheduler has no tasks");
            return Ok(());
        }

        let mut guard = self.scheduler.lock().await;

        let scheduler = guard.as_mut().ok_or_else(|| {
            anyhow::anyhow!("[job] job scheduler not initialized or already shutdown")
        })?;

        scheduler.start().await?;
        self.is_started.store(true, Ordering::SeqCst);
        log::info!("[job] scheduler started with {count} tasks");
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        let mut guard = self.scheduler.lock().await;

        if let Some(scheduler) = guard.as_mut() {
            scheduler.shutdown().await?;
            self.is_started.store(false, Ordering::SeqCst);
            log::info!("[job] scheduler shutdown successfully");
        }
        Ok(())
    }

    pub fn get_task_count(&self) -> usize {
        self.task_count.load(Ordering::SeqCst)
    }

    pub fn is_running(&self) -> bool {
        self.is_started.load(Ordering::SeqCst)
    }
}
