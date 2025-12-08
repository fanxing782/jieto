mod error;

use actix_web::{get, web};
use deadpool_redis::redis::cmd;
use jieto_macros::{scheduled, task};
use jieto_web::{ApiResult, AppInitializing, AppScheduler, AppState, Application, JietoResult};
use serde::Serialize;
use sqlx::FromRow;

#[derive(FromRow, Debug, Serialize)]
pub struct User {
    #[sqlx(rename = "NAME")]
    name: String,
    #[sqlx(rename = "USER")]
    user: String,
}

#[get("/")]
async fn hello(data: web::Data<AppState>) -> JietoResult<User> {
    let pool = data.db_manager.with_mysql_default()?;
    let result = sqlx::query_as::<_, User>(r#"SELECT NAME,USER FROM USER"#)
        .fetch_optional(&pool)
        .await?;

    ApiResult::ok_data(result)
}

#[get("/redis/{key}")]
async fn redis_test(
    data: web::Data<AppState>,
    scheduler: web::Data<AppScheduler>,
    path: web::Path<String>,
) -> JietoResult<String> {
    let key = path.into_inner();
    let pool = data.db_manager.with_redis_default()?;
    let mut conn = pool.get().await.unwrap();
    let result = cmd("GET")
        .arg(&[key])
        .query_async::<String>(&mut conn)
        .await
        .ok();

    println!("task count: {}", scheduler.0.get_task_count());
    ApiResult::ok_data(result)
}

#[scheduled("*/5 * * * * *")]
async fn health_check_task(_state: web::Data<AppState>) {
    println!("Health check running every 5 seconds");
}

#[derive(Default)]
struct ApplicationInit;
impl AppInitializing for ApplicationInit {
    fn initializing(&self) {
        println!("✨ Application initializing...");
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Application::new(|cfg| {
        cfg
            .service(hello)
            .service(redis_test);
    })
    .bind_init(ApplicationInit)
    .register_task(task!(health_check_task))
    .run()
    .await?;
    Ok(())
}
