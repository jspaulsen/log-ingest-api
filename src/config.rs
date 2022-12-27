use envconfig::Envconfig;
use tracing::Level;


#[derive(Clone)]
#[derive(Envconfig)]
pub struct Config {
    #[envconfig(from = "DATABASE_URL")]
    pub database_url: String,

    #[envconfig(from = "HTTP_PORT", default = "8080")]
    pub http_port: u16,

    #[envconfig(from = "HTTP_HOST", default = "0.0.0.0")]
    pub http_host: String,

    #[envconfig(from = "LOG_LEVEL", default = "info")]
    pub log_level: Level,
}
