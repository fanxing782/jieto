use crate::config::ApplicationConfig;
use crate::error::WebError;
use crate::log4r::init_logger;
use actix_cors::Cors;
use actix_web::web::ServiceConfig;
use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use serde::Serialize;
use std::env;
use std::ops::Deref;
use std::sync::Arc;

pub mod config;
pub mod error;
mod log4r;
pub mod resp;

#[cfg(feature = "job")]
pub mod job;
#[cfg(feature = "ws")]
mod ws;

pub use resp::ApiResult;

#[cfg(feature = "job")]
pub type TaskScheduler = jieto_job::TaskScheduler;

#[cfg(feature = "database")]
pub type DbManager = jieto_db::database::DbManager;

#[cfg(not(feature = "job"))]
#[derive(Debug, Clone)]
pub struct TaskScheduler {
    _private: (),
}

#[cfg(feature = "database")]
pub static GLOBAL_DBMANAGER: std::sync::OnceLock<Arc<DbManager>> = std::sync::OnceLock::new();

#[derive(Debug, Clone)]
pub struct BusinessError {
    pub code: u16,
    pub msg: &'static str,
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
    pub db_manager: Arc<DbManager>,
    #[cfg(feature = "ws")]
    pub ws_server: Option<jieto_ws::WsServerHandle>,
}

#[derive(Default, Debug)]
pub struct AppScheduler(#[cfg(feature = "job")] pub Arc<jieto_job::TaskScheduler>);

#[cfg(feature = "database")]
impl AppState {
    fn with_db(&mut self, db_manager: Arc<DbManager>) {
        self.db_manager = db_manager;
    }
}

#[cfg(feature = "ws")]
impl AppState {
    fn with_ws(&mut self, server_tx: jieto_ws::WsServerHandle) {
        self.ws_server = Some(server_tx);
    }
}

#[cfg(feature = "job")]
impl AppScheduler {
    fn with_job(&mut self, scheduler: Arc<jieto_job::TaskScheduler>) {
        self.0 = scheduler;
    }
}

pub trait AppInitializing {
    fn initializing(&self);
}

pub struct Application<I, F>
where
    I: AppInitializing,
    F: Fn(&mut ServiceConfig) + Send + Clone + 'static,
{
    cfg: F,
    init: Vec<I>,
    #[cfg(feature = "job")]
    tasks: Vec<Box<dyn jieto_job::ScheduledTask>>,
}

impl<I, F> Application<I, F>
where
    I: AppInitializing,
    F: Fn(&mut ServiceConfig) + Send + Clone + 'static,
{
    pub fn new(cfg: F) -> Self {
        Self {
            cfg,
            init: vec![],
            #[cfg(feature = "job")]
            tasks: vec![],
        }
    }

    pub fn bind_init(mut self, init: I) -> Self {
        self.init.push(init);
        self
    }

    #[cfg(feature = "job")]
    pub fn register_task(mut self, task: Box<dyn jieto_job::ScheduledTask>) -> Self {
        self.tasks.push(task);
        self
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let config_path = env::var("APP_CONFIG")
            .or_else(|_| env::var("CONFIG_PATH"))
            .unwrap_or_else(|_| "application.toml".to_string()); // 默认路径
        let config = ApplicationConfig::from_toml(&config_path).await?;
        let mut state = AppState::default();
        let mut scheduler = AppScheduler::default();
        init_logger(&config.log, &config.name.unwrap_or(String::from("app")))?;

        #[cfg(feature = "ws")]
        let ws_handle = {
            let (ws_server, server_tx) = jieto_ws::WsServer::new();
            let ws_server_handle = tokio::task::spawn(ws_server.run());
            state.with_ws(server_tx);
            ws_server_handle
        };

        #[cfg(feature = "database")]
        {
            let db_manager = jieto_db::jieto_db_init(&config_path).await?;
            let db_manager = Arc::new(db_manager);
            let db_manager = GLOBAL_DBMANAGER.get_or_init(|| db_manager);
            state.with_db(db_manager.clone());
        }

        while let Some(init) = self.init.pop() {
            init.initializing();
        }

        let app_state = web::Data::new(state);

        #[cfg(feature = "job")]
        {
            let task_scheduler = TaskScheduler::new(Arc::new(app_state.clone())).await?;
            let task_scheduler = Arc::new(task_scheduler);
            while let Some(task) = self.tasks.pop() {
                task_scheduler.register_task(task).await?;
            }
            task_scheduler.start().await?;
            scheduler.with_job(task_scheduler);
        }

        let app_scheduler = web::Data::new(scheduler);
        let cfg_fn = self.cfg.clone();

        let server = HttpServer::new(move || {
            let cors = Cors::default()
                .allow_any_origin() // 允许任意域名（仅开发用！）
                .allow_any_method()
                .allow_any_header()
                .supports_credentials() // 如果需要携带 cookie
                .max_age(3600);

            App::new()
                .app_data(app_state.clone())
                .app_data(app_scheduler.clone())
                .wrap(cors)
                .wrap(actix_web::middleware::Logger::default())
                .configure(|cfg| {
                    #[cfg(feature = "ws")]
                    {
                        use crate::ws::configure_ws;
                        configure_ws(cfg, config.ws.path.as_deref());
                    }

                    cfg_fn(cfg)
                })
        })
        .bind(("0.0.0.0", config.web.port))?
        .run();

        #[cfg(feature = "ws")]
        {
            tokio::try_join!(server, async move { ws_handle.await.unwrap() })?;
        }

        #[cfg(not(feature = "ws"))]
        {
            server.await?;
        }

        Ok(())
    }
}
