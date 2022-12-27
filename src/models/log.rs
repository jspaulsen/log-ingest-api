use sea_orm::{
    DatabaseBackend,
    ConnectionTrait,
    entity::prelude::*,
    Statement, 
    SelectorRaw, 
    SelectModel,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    parameters::{
        FilterParameter,
        Operator,
    },
};


#[derive(DeriveIntoActiveModel, Serialize, Deserialize, Debug, Clone)]
pub struct IngestLog {
    #[serde(default = "IngestLog::default_timestamp")]
    pub timestamp: Option<DateTimeWithTimeZone>,
    pub message: String,
    pub level: i32,

    #[serde(default = "IngestLog::default_context")]
    pub context: Option<Json>,
}

impl IngestLog {
    pub fn default_context() -> Option<Json> {
        Some(Json::Object(serde_json::Map::new()))
    }

    pub fn default_timestamp() -> Option<DateTimeWithTimeZone> {
        Some(
            chrono::Utc::now()
                .into()
        )
    }
}


#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub timestamp: Option<DateTimeWithTimeZone>,
    pub message: String,
    pub level: i32,
    pub context: Option<Json>,
}


#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    // Return a list of column names
    pub fn columns() -> Vec<&'static str> {
        vec!["id", "timestamp", "message", "level", "context"]
    }

    // table name
    pub fn table_name() -> &'static str {
        "logs"
    }

    pub fn query() -> QueryBuilder {
        QueryBuilder::new()
    }
}


fn value_to_cast(value: &sea_orm::Value) -> Option<String> {
    match value {
        sea_orm::Value::Bool(_) => Some("bool".to_string()),
        sea_orm::Value::TinyInt(_) | sea_orm::Value::SmallInt(_) | sea_orm::Value::Int(_) | sea_orm::Value::BigInt(_) |
        sea_orm::Value::TinyUnsigned(_) | sea_orm::Value::SmallUnsigned(_) | sea_orm::Value::Unsigned(_) | 
        sea_orm::Value::BigUnsigned(_) => Some("numeric".to_string()),
        sea_orm::Value::Float(_) | sea_orm::Value::Double(_) => Some("float8".to_string()),
        // datetimewithutc
        sea_orm::Value::ChronoDateTimeUtc(_) => Some("timestampwithtz".to_string()),
        _ => None,
    }
}

fn jsonb_typeof(value: &sea_orm::Value) -> String {
    //object, array, string, number, boolean, and null
    match value {
        sea_orm::Value::Bool(_) => "boolean".to_string(),
        sea_orm::Value::TinyInt(_) | sea_orm::Value::SmallInt(_) | sea_orm::Value::Int(_) | sea_orm::Value::BigInt(_) |
        sea_orm::Value::TinyUnsigned(_) | sea_orm::Value::SmallUnsigned(_) | sea_orm::Value::Unsigned(_) | 
        sea_orm::Value::BigUnsigned(_) => "number".to_string(),
        sea_orm::Value::Float(_) | sea_orm::Value::Double(_) => "number".to_string(),
        sea_orm::Value::String(_) | sea_orm::Value::Char(_) => "string".to_string(),
        sea_orm::Value::Json(_)  => "object".to_string(),
        _ => "string".to_string(),
    }
}

pub struct QueryBuilder {
    sql_statement: Vec<String>,
    order_by: Vec<String>,
    values: Vec<sea_orm::Value>,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self {
            sql_statement: vec![],
            order_by: vec![],
            values: vec![],
        }
    }

    pub fn build(self, db_conn: &DatabaseConnection) -> SelectorRaw<SelectModel<Model>> {
        let db_backend = db_conn.get_database_backend();

        Entity::find()
            .from_raw_sql(self.into_sql_statement(db_backend))
    }

    pub fn into_sql_statement(self, db_backend: DatabaseBackend) -> Statement {
        Statement::from_sql_and_values(
            db_backend,
            &self.raw_sql_statement(),
            self.values,
        )
    }

    pub fn raw_sql_statement(&self) -> String {
        let table_name = Model::table_name();
        let mut statement = format!("SELECT * FROM {}", table_name);

        // Add where statements to query
        if self.sql_statement.len() > 0 {
            statement += " WHERE ";
            statement += self.sql_statement.join(" AND ").as_str();
        }
        
        // Add order by to statement if it exists
        if self.order_by.len() > 0 {
            statement += " ORDER BY ";
            statement += self.order_by.join(", ").as_str();
        }
        
        statement
    }

    pub fn contains<S: Into<String>, T: Into<sea_orm::Value>>(self, field: S, value: T) -> Self {
        self.add_to_sql_statement("LIKE", field, value, true)
    }

    pub fn gt<S: Into<String>, T: Into<sea_orm::Value>>(self, field: S, value: T) -> Self {
        self.add_to_sql_statement(">", field, value, false)
    }

    pub fn gte<S: Into<String>, T: Into<sea_orm::Value>>(self, field: S, value: T) -> Self {
        self.add_to_sql_statement(">=", field, value, false)
    }

    pub fn lt<S: Into<String>, T: Into<sea_orm::Value>>(self, field: S, value: T) -> Self {
        self.add_to_sql_statement("<", field, value, false)
    }

    pub fn lte<S: Into<String>, T: Into<sea_orm::Value>>(self, field: S, value: T) -> Self {
        self.add_to_sql_statement("<=", field, value, false)
    }

    pub fn eq<S: Into<String>, T: Into<sea_orm::Value>>(self, field: S, value: T) -> Self {
        self.add_to_sql_statement("=", field, value, false)
    }

    pub fn order_by_asc<S: Into<String>>(self, column: S) -> Self {
        self.order_by(column, "ASC")
    }

    pub fn order_by_desc<S: Into<String>>(self, column: S) -> Self {
        self.order_by(column, "DESC")
    }

    fn order_by<S: Into<String>>(mut self, column: S, ordering: &str) -> Self {
        let column = column.into();

        if Model::columns().contains(&column.as_str()) {
            self.order_by.push(format!("\"{}\" {}", column, ordering));
            self
        } else {
            self.order_by.push(format!("context->>'{}' {}", column, ordering));

            // add a sql statement checking that the key exists in the json
            self.add_to_sql_statement("?", "context", column, false)
        }
    }

    fn add_to_sql_statement<S: Into<String>, T: Into<sea_orm::Value>>(mut self, operand: &str, field: S, value: T, wildcard: bool) -> Self {
        let positional = self.positional_variable();
        let field = field.into();
        let value = value.into();

        let statement = if Model::columns().contains(&field.as_str()) {
            Self::format_column_statement(&operand, &field, &positional, wildcard)
        } else {
            Self::format_context_statement(&operand, &field, &positional, &value, wildcard)
        };

        self.sql_statement.push(statement.into());
        self.values.push(value.into());

        self
    }

    fn format_column_statement(operand: &str, field: &str, positional: &str, wildcard: bool) -> String {
        let positional = {
            if wildcard {
                format!("'%' || {} || '%'", positional)
            } else {
                positional.to_string()
            }
        };

        format!("\"{}\" {} {}", field, operand, positional)
    }

    fn format_context_statement(operand: &str, field: &str, positional: &str, value: &sea_orm::Value, wildcard: bool) -> String {
        let cast = value_to_cast(&value);
        let typeof_value = jsonb_typeof(&value);
        let typeof_prefix = format!("jsonb_typeof(context->'{}') = '{}'", field, typeof_value);
        let positional = {
            if wildcard {
                format!("'%' || {} || '%'", positional)
            } else {
                positional.to_string()
            }
        };
        
        let query = if let Some(cast) = cast {
            format!("(context->>'{}')::{} {} {}", field, cast, operand, positional)
        } else { // if we don't know what it is, assume it's text
            format!("context->>'{}' {} {}", field, operand, positional)
        };
        
        format!("{} AND {}", typeof_prefix, query)
    }

    fn positional_variable(&self) -> String {
        format!("${}", self.values.len() + 1)
    }
}


impl From<Vec<FilterParameter>> for QueryBuilder {
    fn from(parameters: Vec<FilterParameter>) -> Self {
        let mut query_builder = QueryBuilder::new();

        for parameter in parameters {
            match parameter.op {
                Operator::Eq => query_builder = query_builder.eq(parameter.field, parameter.value),
                Operator::Gt => query_builder = query_builder.gt(parameter.field, parameter.value),
                Operator::Gte => query_builder = query_builder.gte(parameter.field, parameter.value),
                Operator::Lt => query_builder = query_builder.lt(parameter.field, parameter.value),
                Operator::Lte => query_builder = query_builder.lte(parameter.field, parameter.value),
                Operator::Contains => query_builder = query_builder.contains(parameter.field, parameter.value),
            }
        }

        query_builder
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use envconfig::Envconfig;
    use sea_orm::{
        ActiveValue,
        DatabaseBackend,
        DatabaseConnection,
        ConnectionTrait,
        EntityTrait,
        Statement, ModelTrait,
    };
    use serde_json::json;
    
    use crate::{
        config::Config,
        database::{
            get_db_connection,
            self,
        },
    };
    use super::{
        ActiveModel,
        Model,
        QueryBuilder,
    };

    /// Run migrations, setup and truncate any existing data
    async fn setup_db() -> DatabaseConnection {
        let config = {
            let mut hashmap: HashMap<String, String> = HashMap::new();
        
            hashmap.insert(
                "DATABASE_URL".to_string(), 
                "postgres://postgres:postgres@localhost:5432/logs".to_string(),
            );

            Config::init_from_hashmap(&hashmap)
                .unwrap()
        };

        let db_conn = get_db_connection(&config)
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

    #[test]
    fn test_query_builder_ops() {
        let query = QueryBuilder::new()
            .gt("id", 1)
            .lt("id", 10)
            .eq("message", "hello")
            .eq("foo", "bar")
            .raw_sql_statement();
        
        let query_gte_lte = QueryBuilder::new()
            .gte("id", 1)
            .lte("id", 10)
            .eq("message", "hello")
            .raw_sql_statement();
        
        assert_eq!(
            query,
            "SELECT * FROM logs WHERE \"id\" > $1 AND \"id\" < $2 AND \"message\" = $3 AND jsonb_typeof(context->'foo') = 'string' AND context->>'foo' = $4",
        );

        assert_eq!(
            query_gte_lte,
            "SELECT * FROM logs WHERE \"id\" >= $1 AND \"id\" <= $2 AND \"message\" = $3",
        );
    }

    #[test]
    fn test_query_contains() {
        let query = QueryBuilder::new()
            .contains("message", "hello")
            .raw_sql_statement();
        
        assert_eq!(
            query,
            "SELECT * FROM logs WHERE \"message\" LIKE '%' || $1 || '%'",
        );
    }

    #[test]
    fn test_query_buidler_context() {
        let query = QueryBuilder::new()
            .eq("foo", "bar")
            .lte("baz", 5)
            .raw_sql_statement();
        
        assert_eq!(
            query,
            "SELECT * FROM logs WHERE jsonb_typeof(context->'foo') = 'string' AND context->>'foo' = $1 AND jsonb_typeof(context->'baz') = 'number' AND (context->>'baz')::numeric <= $2",
        );
    }

    #[test]
    fn test_query_builder_order_by() {
        let query = QueryBuilder::new()
            .order_by_asc("id")
            .order_by_desc("message")
            .raw_sql_statement();
        
        assert_eq!(
            query,
            "SELECT * FROM logs ORDER BY \"id\" ASC, \"message\" DESC",
        );
    }

    #[test]
    fn test_query_builder_order_by_json() {
        let query = QueryBuilder::new()
            .eq("id", 4)
            .order_by_asc("foo")
            .order_by_desc("bar")
            .raw_sql_statement();
        
        assert_eq!(
            query,
            "SELECT * FROM logs WHERE \"id\" = $1 AND \"context\" ? $2 AND \"context\" ? $3 ORDER BY context->>'foo' ASC, context->>'bar' DESC",
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_model_query() {
        let db = setup_db()
            .await;
        
        let models = vec![
            ActiveModel {
                id: ActiveValue::NotSet,
                timestamp: ActiveValue::NotSet,
                message: ActiveValue::Set("hello".to_string()),
                level: ActiveValue::Set(3),
                context: ActiveValue::Set(
                    Option::Some(
                        json!({
                            "foo": "bar",
                            "baz": "qux",
                            "maybe": 5,
                        })
                    )
                ),
            },
            ActiveModel {
                id: ActiveValue::NotSet,
                timestamp: ActiveValue::NotSet,
                message: ActiveValue::Set("goodbye".to_string()),
                level: ActiveValue::Set(3),
                context: ActiveValue::Set(
                    Option::Some(
                        json!({
                            "foo": "bar",
                            "baz": "qux",
                            "maybe": 7,
                        })
                    )
                ),
            },
            ActiveModel {
                id: ActiveValue::NotSet,
                timestamp: ActiveValue::NotSet,
                message: ActiveValue::Set("goodbye".to_string()),
                level: ActiveValue::Set(3),
                context: ActiveValue::Set(
                    Option::Some(
                        json!({
                            "foo": "bar",
                            "baz": "qux",
                            "maybe": 9,
                        })
                    )
                ),
            }
        ];

        <Model as ModelTrait>::Entity::insert_many(models)
            .exec(&db)
            .await
            .unwrap();
        
        let query = Model::query()
            .gt("maybe", 6)
            .lt("maybe", 10);

        let logs = query
            .build(&db)
            .all(&db)
            .await
            .unwrap();
        
        assert_eq!(logs.len(), 2);
    }
}
