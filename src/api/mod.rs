use axum::{
    routing::get,
    Router,
};
use sea_orm::DatabaseConnection;

use crate::{
    api::logs::Logs,
    config::Config,
};

mod logs;


#[derive(Clone)]
pub struct AppState {
    // Despite the fact every DatabaseConnection wraps an
    // Arc<ConnectionPool>, we need to wrap it in an Arc
    // to satisfy Mock
    pub db: std::sync::Arc<DatabaseConnection>,
    pub config: Config,
}

impl AppState {
    pub fn new(db: DatabaseConnection, config: Config) -> Self {
        Self {
            db: std::sync::Arc::new(db),
            config,
        }
    }
}

pub struct Api {
    db: DatabaseConnection,
    config: Config,
}


impl Api {
    pub fn new(db: DatabaseConnection, config: Config) -> Self {
        Self {
            db,
            config
        }
    }

    pub fn into_router(self) -> Router {
        let state = AppState::new(self.db, self.config);

        Router::new()
            .route(
                "/logs", 
                get(Logs::query_logs)
                    .post(Logs::ingest_logs)
            )
            .with_state(state)
    }
}


impl Into<Router> for Api {
    fn into(self) -> Router {
        self.into_router()
    }
}
