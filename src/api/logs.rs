use std::collections::HashMap;

use axum::{
    extract::{
        Query,
        State,
    },
    http::StatusCode,
    Json,
    response::{
        IntoResponse,
        Response,
    },
};
use sea_orm::{
    EntityTrait,
    IntoActiveModel,
};

use crate::{
    error::{
        HttpError,
        Loggable,
    },
    models::{
        IngestLog,
        Log,
        LogActiveModel,
        QueryBuilder,
    },
    parameters::FilterParameter,
};

use super::AppState;


pub struct Logs;

impl Logs {
    pub async fn query_logs(
        state: State<AppState>,
        Query(params): Query<HashMap<String, String>>
    ) -> Result<Json<serde_json::Value>, HttpError> {
        let filter_parameters = FilterParameter::from_hashmap(params)
            .map_err(|op| HttpError::bad_request(Some(op.to_string())))?;
        let db_connection = state.db.clone();
    
        let results = QueryBuilder::from(filter_parameters)
            .order_by_asc("timestamp")
            .build(&db_connection)
            .into_json()
            .all(&*db_connection)
            .await
            .log_error("An exception occurred while querying logs")
            .map_err(|_| HttpError::internal_server_error(None))?;
    

        Ok(Json(serde_json::Value::Array(results)))
    }

    pub async fn ingest_logs(
        state: State<AppState>,
        Json(logs): Json<Vec<IngestLog>>,
    ) -> Result<Response, HttpError> {
        let db_connection = state.db.clone();
        let count = logs.len();
        let active_logs = logs
            .into_iter()
            .map(|log| log.into_active_model())
            .collect::<Vec<LogActiveModel>>();
        
        Log::insert_many(active_logs)
            .exec(&*db_connection)
            .await
            .log_error("An exception occurred while ingesting logs")
            .map_err(|_| HttpError::internal_server_error(None))?;
        
        let response = (
            StatusCode::ACCEPTED, 
            Json(serde_json::json!({"count": count}))
        ).into_response();

        Ok(response)
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    
    use axum::{
        body::Body,
        http::{
            Request,
            self,
            StatusCode,
        },
    };
    use envconfig::Envconfig;
    use sea_orm::{
        ConnectionTrait,
        DatabaseBackend,
        DatabaseConnection,
        EntityTrait,
        IntoActiveModel,
        MockDatabase,
        MockExecResult, 
        Statement,
    };
    use tower::ServiceExt;

    use crate::{
        api::Api,
        config::Config,
        database,
        models::{
            IngestLog,
            LogActiveModel,
            LogModel,
        },
    };

    fn config() -> Config {
        let mut hashmap: HashMap<String, String> = HashMap::new();
    
        hashmap.insert(
            "DATABASE_URL".to_string(), 
            "postgres://postgres:postgres@localhost:5432/logs".to_string(),
        );

        Config::init_from_hashmap(&hashmap)
            .unwrap()
    }

    async fn setup_db(config: &Config) -> DatabaseConnection {
        let db_conn = database::get_db_connection(&config)
            .await
            .unwrap();

        database::migrate(&config)
            .await
            .unwrap();
        
        db_conn.execute(
            Statement::from_string(
                DatabaseBackend::Postgres,
                "TRUNCATE TABLE logs;".to_owned(),
            )
        ).await
        .unwrap();

        db_conn
    }

    fn to_uri(params: HashMap<String, String>) -> String {
        let uri = "/logs?".to_owned();
        let qs = params
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<Vec<String>>()
            .join("&");

        uri + &qs
    }

    #[tokio::test]
    async fn test_query_logs() {
        let db: DatabaseConnection = MockDatabase::new(DatabaseBackend::MySql)
            .append_query_results(vec![
                vec![LogModel {
                    id: 1,
                    timestamp: None,
                    level: 3,
                    message: "Test message".to_owned(),
                    context: None,
                }],
            ])
            .into_connection();

        let router: axum::Router = Api::new(
            db,
            config(),
        ).into();

        let parameters = [
            ("filter[level][gte]".to_string(), "3".to_string()),
            ("filter[timestamp][gte]".to_string(), "2021-01-01T00:00:00Z".to_string()),
        ];
        
        let request = Request::builder()
            .uri(to_uri(HashMap::from(parameters)))
            .method(http::Method::GET)
            .body(Body::empty())
            .expect("Failed to build request");

        let response = router
            .oneshot(request)
            .await
            .expect("Failed to call API");

        assert_eq!(response.status(), StatusCode::OK);

        // parse the response body
        let body = hyper::body::to_bytes(response.into_body())
            .await
            .expect("Failed to read response body");

        let body: serde_json::Value = serde_json::from_slice(&body)
            .unwrap();

        let arr = body.as_array()
            .expect("Body is in incorrect format");
        
        assert_eq!(arr.len(), 1);

        let message = arr.get(0)
            .expect("Failed to get item from array")
            .get("message")
            .expect("Failed to get message from item")
            .as_str()
            .expect("Failed to get message as string");
        
        assert_eq!(message, "Test message");

    }

    #[tokio::test]
    async fn test_query_logs_fail() {
        let db: DatabaseConnection = MockDatabase::new(DatabaseBackend::MySql)
            .into_connection();

        let router: axum::Router = Api::new(
            db.clone(),
            config(),
        ).into();

        let parameters = [
            ("filter[level][notavalidoperation]".to_string(), "3".to_string()),
        ];
        
        let request = Request::builder()
            .uri(to_uri(HashMap::from(parameters)))
            .method(http::Method::GET)
            .body(Body::empty())
            .expect("Failed to build request");

        let response = router
            .oneshot(request)
            .await
            .expect("Failed to call API");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[ignore]
    #[tokio::test]
    async fn test_query_logs_actual_database() {
        let config = config();
        let db = setup_db(&config)
            .await;

        // insert items into database
        let models: Vec<IngestLog> = vec![
            IngestLog {
                timestamp: None,
                level: 3,
                message: "Test message".to_owned(),
                context: Some(serde_json::json!({
                    "test": "test"
                })),
            },
            IngestLog {
                timestamp: None,
                level: 2,
                message: "Shouldn't Be Seen".to_owned(),
                context: Some(serde_json::json!({
                    "test": "test"
                })),
            },
        ];

        let active_models: Vec<LogActiveModel> = models
            .into_iter()
            .map(|model| model.into_active_model())
            .collect();
        
        crate::models::Log::insert_many(active_models)
            .exec(&db)
            .await
            .unwrap();

        let router: axum::Router = Api::new(
            db,
            config,
        ).into();

        let parameters = [
            ("filter[level][gte]".to_string(), "3".to_string()),
        ];

        let request = Request::builder()
            .uri(to_uri(HashMap::from(parameters)))
            .method(http::Method::GET)
            .body(Body::empty())
            .expect("Failed to build request");

        let response = router
            .oneshot(request)
            .await
            .expect("Failed to call API");

        assert_eq!(response.status(), StatusCode::OK);

        // parse the response body
        let body = hyper::body::to_bytes(response.into_body())
            .await
            .expect("Failed to read response body");

        let body: serde_json::Value = serde_json::from_slice(&body)
            .unwrap();

        let arr = body.as_array()
            .expect("Body is in incorrect format");
        
        assert_eq!(arr.len(), 1);

    }

    #[tokio::test]
    async fn test_ingest_mocked() {
        let db: DatabaseConnection = MockDatabase::new(DatabaseBackend::MySql)
            .append_exec_results(vec![
                MockExecResult {
                    last_insert_id: 15,
                    rows_affected: 2,
                },
            ]
        ).into_connection();

        let router: axum::Router = Api::new(
            db,
            config(),
        ).into();

        let body = serde_json::json!([
            {
                "level": 3,
                "message": "Test message",
                "context": {
                    "test": "test"
                }
            },
            {
                "timestamp": "2021-01-01T00:00:00Z",
                "level": 2,
                "message": "Test message 2",
                "context": {
                    "test": "test"
                }
            }
        ]);

        let request = Request::builder()
            .uri("/logs")
            .method(http::Method::POST)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .expect("Failed to build request");

        let response = router
            .oneshot(request)
            .await
            .expect("Failed to call API");

        assert_eq!(response.status(), StatusCode::ACCEPTED);

        // parse the response body
        let body = hyper::body::to_bytes(response.into_body())
            .await
            .expect("Failed to read response body");

        let body: serde_json::Value = serde_json::from_slice(&body)
            .unwrap();
        
        assert_eq!(body.get("count").unwrap().as_u64().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_ingest_fail() {
        let db: DatabaseConnection = MockDatabase::new(DatabaseBackend::MySql)
            .into_connection();

        let router: axum::Router = Api::new(
            db,
            config(),
        ).into();

        // missing field
        let body = serde_json::json!([
            {
                "level": 3,
                "context": {
                    "test": "test"
                }
            },
        ]);

        let request = Request::builder()
            .uri("/logs")
            .method(http::Method::POST)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .expect("Failed to build request");

        let response = router
            .oneshot(request)
            .await
            .expect("Failed to call API");

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[ignore]
    #[tokio::test]
    async fn test_ingest_database() {
        let config = config();
        let db = setup_db(&config)
            .await;

        let router: axum::Router = Api::new(
            db,
            config.clone(),
        ).into();

        let body = serde_json::json!([
            {
                "level": 3,
                "message": "Test message",
                "context": {
                    "test": "test"
                }
            },
            {
                "timestamp": "2021-01-01T00:00:00Z",
                "level": 2,
                "message": "Test message 2",
                "context": {
                    "test": "test"
                }
            }
        ]);

        let request = Request::builder()
            .uri("/logs")
            .method(http::Method::POST)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .expect("Failed to build request");

        let response = router
            .oneshot(request)
            .await
            .expect("Failed to call API");

        assert_eq!(response.status(), StatusCode::ACCEPTED);

        // parse the response body
        let body = hyper::body::to_bytes(response.into_body())
            .await
            .expect("Failed to read response body");

        let body: serde_json::Value = serde_json::from_slice(&body)
            .unwrap();
        
        assert_eq!(body.get("count").unwrap().as_u64().unwrap(), 2);

        let db_conn = database::get_db_connection(&config)
            .await
            .unwrap();

        let query = crate::models::QueryBuilder::new()
            .build(&db_conn)
            .all(&db_conn)
            .await
            .unwrap();
        
        assert_eq!(query.len(), 2);
    }
}
