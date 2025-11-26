use crate::task::ScheduledTask;
use anyhow::Result;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::OnceCell;
use tokio_cron_scheduler::{Job, JobScheduler};

pub struct TaskScheduler {
    scheduler: OnceCell<JobScheduler>,
    task_count: AtomicUsize,
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self {
            scheduler: OnceCell::new(),
            task_count: AtomicUsize::new(0),
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
    pub async fn new() -> Result<Self> {
        let job_scheduler_cell = OnceCell::new();
        let job_scheduler = JobScheduler::new().await?;
        job_scheduler_cell
            .set(job_scheduler)
            .map_err(|_| anyhow::anyhow!("[job] failed to initialize job scheduler"))?;

        Ok(Self {
            scheduler: job_scheduler_cell,
            task_count: AtomicUsize::new(0),
        })
    }

    pub async fn register_task(&self, task: Box<dyn ScheduledTask>) -> Result<()> {
        let scheduler = self
            .scheduler
            .get()
            .ok_or_else(|| anyhow::anyhow!("[job] job scheduler not initialized"))?;

        let cron_expr = task.cron_expression().to_string();
        let task_name = task.task_name().to_string();

        log::info!(
            "[job] registering task: {} with cron: {}",
            task_name,
            cron_expr
        );

        // Wrap task in Arc for sharing across async boundaries
        let task = Arc::new(task);

        let job = Job::new_async(cron_expr.as_str(), move |_uuid, _lock| {
            let task = Arc::clone(&task);
            Box::pin(async move {
                log::debug!("️[job] [{}] starting execution...", task.task_name());
                task.execute().await;
                log::debug!("[job] [{}] completed execution", task.task_name());
            })
        })?;

        scheduler.add(job).await?;
        self.task_count.fetch_add(1, Ordering::SeqCst);

        log::info!("[job] successfully registered task: {}", task_name);
        Ok(())
    }

    pub async fn start(&self) -> Result<()> {
        let scheduler = self
            .scheduler
            .get()
            .ok_or_else(|| anyhow::anyhow!("[job] job scheduler not initialized"))?;

        scheduler.start().await?;

        log::info!(
            "[job] scheduler started with {} tasks",
            self.task_count.load(Ordering::SeqCst)
        );
        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(scheduler) = self.scheduler.get_mut() {
            scheduler.shutdown().await?;
            log::info!("[job] scheduler shutdown successfully");
        }
        Ok(())
    }

    pub fn get_task_count(&self) -> usize {
        self.task_count.load(Ordering::SeqCst)
    }
}
