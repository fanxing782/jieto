use std::future::Future;
use std::pin::Pin;

pub trait ScheduledTask: Send + Sync {
    /// Returns the cron expression for this task
    fn cron_expression(&self) -> &'static str;

    /// Returns the name of this task
    fn task_name(&self) -> &'static str;

    /// Executes the task logic
    fn execute(&self) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
}
