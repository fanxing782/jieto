mod error;

use actix_web::{ get, web};
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
    let result =
        sqlx::query_as::<_, User>(r#"SELECT NAME,USER FROM USER"#)
            .fetch_optional(&pool)
            .await?;
    ApiResult::ok_data(result)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    jieto_web_start("application.toml", ||async {

    },|cfg| {
        cfg.service(hello);
    })
    .await?;
    Ok(())
}
