use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub trait ScheduledTask: Send + Sync {
    /// Returns the cron expression for this task
    fn cron_expression(&self) -> &'static str;

    /// Returns the name of this task
    fn task_name(&self) -> &'static str;

    /// Executes the task logic
    fn execute(
        &self,
        injected: Arc<dyn Any + Send + Sync>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
}
