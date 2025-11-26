mod error;

use actix_web::{get, web};
use deadpool_redis::redis::cmd;
// use jieto_web::job::{ScheduledTask, scheduled, task};
use jieto_web::{ApiResult, AppState, JietoResult, jieto_web_start};
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
async fn redis_test(data: web::Data<AppState>, path: web::Path<String>) -> JietoResult<String> {
    let key = path.into_inner();
    let pool = data.db_manager.with_redis_default()?;
    let mut conn = pool.get().await.unwrap();
    let result = cmd("GET")
        .arg(&[key])
        .query_async::<String>(&mut conn)
        .await
        .ok();

    ApiResult::ok_data(result)
}

// #[scheduled("*/5 * * * * *")]
// async fn health_check_task() {
//     println!("Health check running every 5 seconds");
// }

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    jieto_web_start(
        "application.toml",
        || {
            Box::pin(async {
                println!("✨ Application initializing...");
            })
        },
        |cfg| {
            cfg.service(hello).service(redis_test);
        },
        |scheduler| async move {
            // scheduler.register_task(task!(health_check_task)).await.ok();
        },
    )
    .await?;
    Ok(())
}
