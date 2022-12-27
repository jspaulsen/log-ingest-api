use std::{
    collections::HashMap,
    error::Error,
    fmt::{
        Display, 
        Formatter,
    },
    str::FromStr, 
};
use axum::http::StatusCode;
use sea_orm::Value;


/// "Guesses" a value type from a string parameter
/// by bruteforcing the possible types.
struct Type {
    value: String,
}

impl From<String> for Type {
    fn from(value: String) -> Self {
        Self {
            value,
        }
    }
}

impl Type {
    pub fn into_value(self) -> Value {
        if let Ok(value) = self.value.parse::<i64>() {
            Value::BigInt(Some(value))
        } else if let Ok(value) = self.value.parse::<f64>() {
            Value::Double(Some(value))
        } else if self.value.to_lowercase() == "true" {
            Value::Bool(Some(true))
        } else if let Ok(value) = chrono::DateTime::parse_from_rfc3339(&self.value) {
            Value::ChronoDateTimeUtc(Some(Box::new(value.with_timezone(&chrono::Utc))))
        } else if self.value.to_lowercase() == "false" {
            Value::Bool(Some(false))
        } else {
            Value::String(Some(Box::new(self.value)))
        }
    }
}

impl Into<Value> for Type {
    fn into(self) -> Value {
        self.into_value()
    }
}


#[derive(Debug)]
pub struct OperatorParserError {
    operator: String,
}

impl Error for OperatorParserError {}

impl Display for OperatorParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Could not parse operation: {}", self.operator)
    }
}

impl OperatorParserError {
    pub fn from(operator: String) -> Self {
        Self {
            operator,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Operator {
    Contains,
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl FromStr for Operator {
    type Err = OperatorParserError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "contains" => Ok(Self::Contains),
            "eq" => Ok(Self::Eq),
            "gt" => Ok(Self::Gt),
            "gte" => Ok(Self::Gte),
            "lt" => Ok(Self::Lt),
            "lte" => Ok(Self::Lte),
            _ => Err(OperatorParserError::from(s.to_string())),
        }
    }
}

#[derive(Debug)]
pub struct FilterParameterError {
    filter: String,
}

impl Error for FilterParameterError {}

impl Display for FilterParameterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid FilterParameter: {}", self.filter)
    }
}

impl Into<StatusCode> for FilterParameterError {
    fn into(self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

impl FilterParameterError {
    pub fn from(filter: String) -> Self {
        Self {
            filter,
        }
    }
}

pub struct FilterParameter {
    pub field: String,
    pub op: Operator,
    pub value: Value,
}

impl FilterParameter {
    pub fn from_hashmap(hashmap: HashMap<String, String>) -> Result<Vec<Self>, FilterParameterError> {
        hashmap
            .into_iter()
            .map(|(key, value)| {
                Self::parse(
                    key,
                    Type::from(value)
                )
            }).collect()
    }

    pub fn parse<I: Into<Value>>(filter: String, value: I) -> Result<Self, FilterParameterError> {
        let split: Vec<&str> = filter.split("[")
            .map(|s| 
                s.strip_suffix("]")
                    .unwrap_or(s)
            )
            .take(3)
            .collect();
        
        if let [fstr, field, op] = &split[..] {
            let op = op.parse::<Operator>()
                .map_err(|_| FilterParameterError::from(filter.clone()))?;
            
            if *fstr != "filter" {
                return Err(FilterParameterError::from(filter));
            }

            Ok(Self {
                field: field.to_string(),
                op: op,
                value: value.into(),
            })
        } else { // invalid filter
            Err(FilterParameterError::from(filter))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Value;

    #[test]
    fn test_filter_parameter_parse() {
        let filter = "filter[foo][eq]".to_string();
        let value = Value::String(Some(Box::new("bar".to_string())));

        let filter_param = FilterParameter::parse(filter, value)
            .unwrap();

        assert_eq!(filter_param.field, "foo");
        assert_eq!(filter_param.op, Operator::Eq);
    }

    #[test]
    fn test_failing_filter_parameter_parse() {
        let filter1 = "falter[foo][eq]".to_string();
        let filter2 = "filter[foo][notanop]".to_string();
        let value = Value::String(Some(Box::new("bar".to_string())));

        let filter_param1 = FilterParameter::parse(filter1, value.clone());
        let filter_param2 = FilterParameter::parse(filter2, value);

        assert!(filter_param1.is_err());
        assert!(filter_param2.is_err());
    }
}
