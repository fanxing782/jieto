use crate::config::{ApplicationConfig, Log};
use actix_web::web::ServiceConfig;
use actix_web::{App, HttpResponse, HttpServer, Responder, ResponseError, web};
use flexi_logger::{
    Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming, WriteMode, detailed_format,
};
use serde::Serialize;
use thiserror::Error;
use std::sync::OnceLock;
use jieto_db::database::DbManager;

mod config;

#[cfg(feature = "database")]
pub static GLOBAL_DBMANAGER:OnceLock<DbManager> = OnceLock::new();



#[derive(Debug, Clone)]
pub struct BusinessError {
    pub code: u16,
    pub msg: &'static str,
}

#[derive(Serialize, Default, Clone, Debug)]
pub struct ApiResult<T>
where
    T: Serialize,
{
    pub code: u16,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> Responder for ApiResult<T>
where
    T: Serialize,
{
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, _req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().json(self)
    }
}

impl<T> ApiResult<T>
where
    T: Serialize,
{
    const SUCCESS_CODE: u16 = 0;

    pub fn ok(data: T) -> JietoResult<T> {
        Ok(ApiResult {
            code: Self::SUCCESS_CODE,
            msg: "success".to_string(),
            data: Some(data),
        })
    }

    pub fn ok_data(data: Option<T>) -> JietoResult<T> {
        Ok(ApiResult {
            code: Self::SUCCESS_CODE,
            msg: "success".to_string(),
            data,
        })
    }

    pub fn ok_custom(msg: &str, data: Option<T>) -> JietoResult<T> {
        Ok(ApiResult {
            code: Self::SUCCESS_CODE,
            msg: msg.to_string(),
            data,
        })
    }

    pub fn ok_empty() -> JietoResult<T> {
        Ok(ApiResult {
            code: Self::SUCCESS_CODE,
            msg: "success".to_string(),
            data: None,
        })
    }

    pub fn error(business_error: &BusinessError) -> JietoResult<T> {
        Err(WebError::Business(
            business_error.code,
            String::from(business_error.msg),
        ))
    }

    pub fn error_custom(code: u16, msg: &str) -> JietoResult<()> {
        Err(WebError::Business(code, String::from(msg)))
    }
}

#[derive(Error, Debug)]
pub enum WebError {
    #[error("[Business]:{0}")]
    Business(u16, String),
    #[error("[WEB]:{0}")]
    Web(#[from] actix_web::Error),
    #[cfg(feature = "database")]
    #[error("[DB]:{0}")]
    DataSource(#[from] jieto_db::error::DbError),
    #[cfg(feature = "database")]
    #[error("[EX]:{0}")]
    Execution(#[from] sqlx::Error),
}

impl ResponseError for WebError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            WebError::Web(_) | WebError::DataSource(_) | WebError::Execution(_) => {
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            }
            WebError::Business(..) => actix_web::http::StatusCode::default(),
        }
    }
    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let (code, msg) = match self {
            WebError::Business(biz_code, msg) => (*biz_code, msg.clone()),
            _ => (status.as_u16(), format!("{}", self)),
        };

        let res = ApiResult::<()> {
            code,
            msg,
            data: None,
        };

        HttpResponse::build(status).json(res)
    }
}

pub type JietoResult<T> = Result<ApiResult<T>, WebError>;

pub struct Success<T>(pub T);

impl<T> Responder for Success<T>
where
    T: Serialize,
{
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, _req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        let res = ApiResult {
            code: 200,
            msg: "success".to_string(),
            data: Some(self.0),
        };
        HttpResponse::Ok().json(res)
    }
}

#[derive(Default, Debug)]
pub struct AppState {
    #[cfg(feature = "database")]
    pub db_manager: jieto_db::database::DbManager,
}

#[cfg(feature = "database")]
impl AppState {
    fn with_db(&mut self, db_manager:  jieto_db::database::DbManager) {
        self.db_manager = db_manager;
    }
}

fn parse_age(age_str: &str) -> Option<Age> {
    match age_str.to_lowercase().as_str() {
        "second" => Some(Age::Second),
        "minute" => Some(Age::Minute),
        "hour" => Some(Age::Hour),
        "day" => Some(Age::Day),
        _ => None,
    }
}

fn init_logger(config: &Log, app_name: &str) -> anyhow::Result<()> {
    let mut filespec = FileSpec::default().suffix("log");

    // 设置目录
    if let Some(dir) = &config.directory {
        filespec = filespec.directory(dir);
    } else {
        filespec = filespec.directory("logs");
    }

    // 设置 basename（即文件名前缀）
    let basename = config.filename_prefix.as_deref().unwrap_or(app_name);
    filespec = filespec.basename(basename);

    // 构建滚动策略
    let criterion = match (&config.age, config.max_size_mb) {
        (Some(age_str), Some(size_mb)) => {
            if let Some(age) = parse_age(age_str) {
                Criterion::AgeOrSize(age, size_mb * 1024 * 1024)
            } else {
                eprintln!("无效的 age 值: '{}', 默认使用 Size 滚动", age_str);
                Criterion::Size(size_mb * 1024 * 1024)
            }
        }
        (Some(age_str), None) => {
            if let Some(age) = parse_age(age_str) {
                Criterion::Age(age)
            } else {
                eprintln!("无效的 age 值: '{}', 默认不滚动", age_str);

                Criterion::Size(100 * 1024 * 1024) // 100 MB
            }
        }
        (None, Some(size_mb)) => Criterion::Size(size_mb * 1024 * 1024),
        (None, None) => {
            Criterion::Size(10 * 1024 * 1024 * 1024) // 10 GB
        }
    };

    let default_level = String::from("info");
    let level = config.level.as_ref().unwrap_or(&default_level);
    let _ = Logger::try_with_str(level)?
        .log_to_file(filespec)
        .rotate(
            criterion,
            Naming::TimestampsDirect,
            Cleanup::KeepLogFiles(config.keep_files),
        )
        .write_mode(WriteMode::BufferAndFlush)
        .duplicate_to_stderr(Duplicate::All)
        .format_for_files(detailed_format)
        .start()?;

    Ok(())
}

pub async fn jieto_web_start<F,Init,Fut>(path: &str, init:Init , configure_fn: F) -> anyhow::Result<()>
where
    F: Fn(&mut ServiceConfig) + Clone + Send + Sync + 'static,
    Init: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let config = ApplicationConfig::from_toml(path).await?;

    let mut state = AppState::default();

    init_logger(&config.log, &config.name.unwrap_or(String::from("app")))?;

    #[cfg(feature = "database")]
    {
        let db_manager = jieto_db::jieto_db_init("db.toml").await?;
        let db_manager = GLOBAL_DBMANAGER.get_or_init(|| db_manager);
        state.with_db(db_manager.clone());
    }

    init().await;

    let app_state = web::Data::new(state);
    let _server = HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(actix_web::middleware::Logger::default())
            .configure(|cfg| configure_fn(cfg))
    })
    .bind(("0.0.0.0", config.web.port))?
    .run()
    .await;
    Ok(())
}
