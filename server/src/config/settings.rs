use config::{Config, ConfigError, Environment, File};
use kubuno_storage::StorageConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server:   ServerSettings,
    pub core:     CoreSettings,
    pub database: DatabaseSettings,
    pub storage:  StorageConfig,
    pub photos:   PhotosSettings,
    pub logging:  LoggingSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoreSettings {
    pub url:             String,
    pub internal_secret: String,
}

/// The `[database]` section is owned by kubuno-db: which of its fields matter
/// depends on the engine the administrator selects at run time, and the pool is
/// opened by `kubuno_db::connect`.
pub use kubuno_db::DbSettings as DatabaseSettings;

#[derive(Debug, Clone, Deserialize)]
pub struct PhotosSettings {
    pub max_upload_bytes: u64,
    pub thumbnail_size:   u32,
    pub preview_size:     u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingSettings {
    pub level:  String,
    pub format: LogFormat,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Pretty,
    Json,
}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        let mut builder = Config::builder()
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 3103i64)?
            .set_default("core.url", "http://127.0.0.1:8080")?
            .set_default("core.internal_secret", "")?
            .set_default("database.engine", "postgres")?
            .set_default("database.host", "localhost")?
            .set_default("database.port", 5432i64)?
            .set_default("database.user", "kubuno")?
            .set_default("database.password", "")?
            .set_default("database.database", "kubuno")?
            // SQLite only: the directory that holds `<schema>.sqlite`.
            .set_default("database.path", "./data/db")?
            .set_default("database.max_connections", 10i64)?
            .set_default("database.min_connections", 1i64)?
            .set_default("database.connect_timeout", 10i64)?
            .set_default("database.run_migrations", true)?
            .set_default("storage.backend", "local")?
            .set_default("storage.local_path", "./data/photos")?
            .set_default("storage.temp_path", "./data/temp")?
            .set_default("photos.max_upload_bytes", 104_857_600u64)?
            .set_default("photos.thumbnail_size", 400u64)?
            .set_default("photos.preview_size", 1200u64)?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "pretty")?
            .add_source(File::with_name("config").required(false))
            .add_source(File::with_name("/etc/kubuno/modules/photos/config").required(false))
            .add_source(
                Environment::with_prefix("KP")
                    .separator("__")
                    .try_parsing(true),
            );

        // Variables injectées par le superviseur core — priorité maximale
        if let Ok(v) = std::env::var("KUBUNO_CORE_URL")        { builder = builder.set_override("core.url",             v)?; }
        if let Ok(v) = std::env::var("KUBUNO_INTERNAL_SECRET") { builder = builder.set_override("core.internal_secret", v)?; }
        if let Ok(v) = std::env::var("KUBUNO_DB_HOST")         { builder = builder.set_override("database.host",     v)?; }
        if let Ok(v) = std::env::var("KUBUNO_DB_PORT")         { builder = builder.set_override("database.port",     v.parse::<i64>().unwrap_or(5432))?; }
        if let Ok(v) = std::env::var("KUBUNO_DB_USER")         { builder = builder.set_override("database.user",     v)?; }
        if let Ok(v) = std::env::var("KUBUNO_DB_PASSWORD")     { builder = builder.set_override("database.password", v)?; }
        if let Ok(v) = std::env::var("KUBUNO_DB_NAME")         { builder = builder.set_override("database.database", v)?; }
        if let Ok(v) = std::env::var("KUBUNO_DB_ENGINE")       { builder = builder.set_override("database.engine",   v)?; }
        if let Ok(v) = std::env::var("KUBUNO_DB_PATH")         { builder = builder.set_override("database.path",     v)?; }

        builder.build()?.try_deserialize()
    }
}
