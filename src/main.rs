use envconfig::Envconfig;

use config::Config;

mod api;
mod config;
mod database;
mod error;
mod models;
mod parameters;


#[tokio::main]
async fn main() {
    let config = Config::init_from_env()
        .expect("Failed to load configuration!");

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            config
                .log_level
                .to_string()
        )
        .with_current_span(false)
        .init();

    let db_connection = database::get_db_connection(&config)
        .await
        .expect("Failed to setup database connection!");
    
    // run database migrations as part of application startup
    database::migrate(&config)
        .await
        .expect("Failed to run database migrations!");

    let bind_to = format!("{}:{}", config.http_host, config.http_port);
    let api = api::Api::new(db_connection, config);

    axum::Server::bind(
        &bind_to
            .parse()
            .expect("Invalid bind address!"),
    ).serve(
        api.into_router()
            .into_make_service()
    ).await
    .unwrap();
}
