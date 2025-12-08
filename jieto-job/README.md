# jieto-job

## 使用方法

声明定时任务

```rust
#[scheduled("*/5 * * * * *")]
async fn health_check_task(state: web::Data<AppState>) {
    
}
```

注册定时任务

`register_task(task!(health_check_task))`

```rust
use jieto_macros::{scheduled, task};
use jieto_web::job::ScheduledTask;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Application::new(|cfg| {
        ...
    })
        .register_task(task!(health_check_task))
        .run()
        .await?;
    Ok(())
}
```