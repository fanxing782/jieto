use serde::Deserialize;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[derive(Deserialize, Debug, Default)]
pub(crate) struct ApplicationConfig {
    pub name: Option<String>,
    pub web: Web,
    pub log: Log,
    #[cfg(feature = "ws")]
    pub ws: Ws,
}

#[derive(Deserialize, Debug, Default)]
pub(crate) struct Web {
    pub port: u16,
}

#[derive(Deserialize, Debug, Default)]
pub(crate) struct Log {
    #[serde(default)]
    pub directory: Option<String>,
    #[serde(default)]
    pub filename_prefix: Option<String>,
    pub max_size_mb: Option<u64>,
    pub age: Option<String>,
    pub keep_files: usize,
    pub level: Option<String>,
}

#[cfg(feature = "ws")]
#[derive(Deserialize, Debug, Default)]
pub(crate) struct Ws {
    #[serde(default)]
    pub path: Option<String>,
    // pub heartbeat_interval: Option<u64>,
}

impl ApplicationConfig {
    pub(crate) async fn from_toml(path: &str) -> anyhow::Result<Self> {
        let mut file = File::open(path).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;
        let config: ApplicationConfig = toml::from_str(&contents)?;
        Ok(config)
    }
}
