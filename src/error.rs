use axum::{
    http::StatusCode,
    Json,
    response::{
        IntoResponse,
        Response,
    },
};

use serde::Serialize;
use serde_json::json;


#[derive(Debug, Clone, Serialize)]
pub enum HttpError {
    BadRequest(String),
    InternalServerError(String),
}


pub trait Loggable<T, E> {
    fn log_error<C>(self, context: C) -> Result<T, E>
    where
        C: std::fmt::Display + Send + Sync + 'static;
}
// pub trait Loggable<T, E>: Context<T, E> {
//     fn log_error<C>(self, context: C) -> Result<T, AnyhowError>
//     where
//         C: std::fmt::Display + Send + Sync + 'static;

// }


impl<T, E> Loggable<T, E> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static {

    fn log_error<C: std::fmt::Display + Send + Sync + 'static>(self, context: C) -> Result<T, E> {
        self
            .map_err(|e| {
                tracing::error!("{} {:#?}", context, e);

                e
            })
    }
}


impl HttpError {
    pub fn internal_server_error(message: Option<String>) -> Self {
        let message: String = message
            .unwrap_or("Internal Server Error".to_string());

        Self::InternalServerError(message)
    }

    pub fn bad_request(message: Option<String>) -> Self {
        let message: String = message
            .unwrap_or("Bad Request".to_string());

        Self::BadRequest(message)
    }
}


impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status_code, message) = match self {
            HttpError::InternalServerError(s) => (StatusCode::INTERNAL_SERVER_ERROR, s),
            HttpError::BadRequest(s) => (StatusCode::BAD_REQUEST, s),
        };

        (status_code, Json(json!({"message": message}))).into_response()
    }
}


// impl From<AuthBasicError> for HttpError {
//     fn from(err: AuthBasicError) -> Self {
//         Self::bad_request(Some(err.to_string()))
//     }
// }


// impl From<SqlxError> for HttpError {
//     fn from(_: SqlxError) -> Self {
//         Self::bad_gateway(None)
//     }
// }


// impl From<anyhow::Error> for HttpError {
//     fn from(_: anyhow::Error) -> Self {
//         Self::internal_server_error(None)
//     }
// }
