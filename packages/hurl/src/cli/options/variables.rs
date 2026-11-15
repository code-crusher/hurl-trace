/*
 * Hurl (https://hurl.dev)
 * Copyright (C) 2026 Orange
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */
use hurl_core::ast::is_variable_reserved;

use super::CliOptionsError;
use crate::runner::{Number, Value};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TypeKind {
    /// Variables type inferred from value
    Inferred,
    /// Variables are forced to be string
    String,
}

/// Parses a string "name=value" as a pair of `String` and `Value`.
///
/// If `type_kind` is `TypeKind::Inferred`, value variant is inferred from the `value`,
/// for instance `true` is parsed as [`Value::Bool(true)`].
/// If `type_kind` is `TypeKind::String`, value is parsed as [`Value::String`]
pub fn parse(s: &str, type_kind: TypeKind) -> Result<(String, Value), CliOptionsError> {
    match s.find('=') {
        None => Err(CliOptionsError::Error(format!(
            "Missing value for variable {s}!"
        ))),
        Some(index) => {
            let (name, value) = s.split_at(index);
            if is_variable_reserved(name) {
                return Err(CliOptionsError::Error(format!(
                    "Variable {name} conflicts with the {name} function, use a different name."
                )));
            }
            let value = parse_value(&value[1..], type_kind)?;
            Ok((name.to_string(), value))
        }
    }
}

/// Parses a `NAME=VALUE1,VALUE2,...` string as a job variable.
///
/// The values are separated by commas. A value that contains a comma can be double-quoted.
/// An empty value list (eg `NAME=`) is an error, as is an empty value between commas
/// (eg `NAME=a,,b`); use `""` to define an empty string value.
///
/// If `type_kind` is `TypeKind::Inferred`, each value variant is inferred from the `value`,
/// for instance `true` is parsed as [`Value::Bool(true)`].
/// If `type_kind` is `TypeKind::String`, each value is parsed as [`Value::String`]
pub fn parse_job_variable(
    s: &str,
    type_kind: TypeKind,
) -> Result<(String, Vec<Value>), CliOptionsError> {
    match s.find('=') {
        None => Err(CliOptionsError::Error(format!(
            "Missing value for job variable {s}!"
        ))),
        Some(index) => {
            let (name, value) = s.split_at(index);
            if is_variable_reserved(name) {
                return Err(CliOptionsError::Error(format!(
                    "Variable {name} conflicts with the {name} function, use a different name."
                )));
            }
            let raw_values = &value[1..];
            if raw_values.trim().is_empty() {
                return Err(CliOptionsError::Error(format!(
                    "Missing value for job variable {name}!"
                )));
            }
            let mut values = Vec::new();
            for raw_value in split_values(raw_values) {
                if raw_value.is_empty() {
                    return Err(CliOptionsError::Error(format!(
                        "Empty value in job variable {name}"
                    )));
                }
                values.push(parse_value(&raw_value, type_kind)?);
            }
            Ok((name.to_string(), values))
        }
    }
}

/// Splits a comma-separated list of values, ignoring commas inside double quotes.
fn split_values(s: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            ',' if !in_quotes => {
                values.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    values.push(current.trim().to_string());
    values
}

/// Parses a `value` as a pair of String and Value.
///
/// If `inferred` is `true`, value variant is inferred from the `value`, for instance true is parsed as [`Value::Bool(true)`].
pub fn parse_value(s: &str, type_kind: TypeKind) -> Result<Value, CliOptionsError> {
    if type_kind == TypeKind::String {
        Ok(Value::String(s.to_string()))
    } else if s == "true" {
        Ok(Value::Bool(true))
    } else if s == "false" {
        Ok(Value::Bool(false))
    } else if s == "null" {
        Ok(Value::Null)
    } else if let Ok(v) = s.parse::<i64>() {
        Ok(Value::Number(Number::Integer(v)))
    } else if s.chars().all(char::is_numeric) {
        Ok(Value::Number(Number::BigInteger(s.to_string())))
    } else if let Ok(v) = s.parse::<f64>() {
        Ok(Value::Number(Number::Float(v)))
    } else if let Some(s) = s.strip_prefix('"') {
        if let Some(s) = s.strip_suffix('"') {
            Ok(Value::String(s.to_string()))
        } else {
            Err(CliOptionsError::Error(
                "Value should end with a double quote".to_string(),
            ))
        }
    } else {
        Ok(Value::String(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::{CliOptionsError, *};

    #[test]
    fn test_parse() {
        assert_eq!(
            parse("name=Jennifer", TypeKind::Inferred).unwrap(),
            ("name".to_string(), Value::String("Jennifer".to_string()))
        );
        assert_eq!(
            parse("female=true", TypeKind::Inferred).unwrap(),
            ("female".to_string(), Value::Bool(true))
        );
        assert_eq!(
            parse("age=30", TypeKind::Inferred).unwrap(),
            ("age".to_string(), Value::Number(Number::Integer(30)))
        );
        assert_eq!(
            parse("height=1.7", TypeKind::Inferred).unwrap(),
            ("height".to_string(), Value::Number(Number::Float(1.7)))
        );
        assert_eq!(
            parse("id=\"123\"", TypeKind::Inferred).unwrap(),
            ("id".to_string(), Value::String("123".to_string()))
        );
        assert_eq!(
            parse("id=9223372036854775808", TypeKind::Inferred).unwrap(),
            (
                "id".to_string(),
                Value::Number(Number::BigInteger("9223372036854775808".to_string()))
            )
        );
        assert_eq!(
            parse("a_null=null", TypeKind::Inferred).unwrap(),
            ("a_null".to_string(), Value::Null)
        );
        assert_eq!(
            parse("a_null=null", TypeKind::String).unwrap(),
            ("a_null".to_string(), Value::String("null".to_string()))
        );
    }

    #[test]
    fn test_parse_error() {
        assert_eq!(
            parse("name", TypeKind::Inferred).err().unwrap(),
            CliOptionsError::Error("Missing value for variable name!".to_string())
        );
    }

    #[test]
    fn test_parse_job_variable() {
        let (name, values) =
            parse_job_variable("SUFFIX=red,green,blue", TypeKind::Inferred).unwrap();
        assert_eq!(name, "SUFFIX");
        assert_eq!(
            values,
            vec![
                Value::String("red".to_string()),
                Value::String("green".to_string()),
                Value::String("blue".to_string()),
            ]
        );

        // Values are typed with the inferred type.
        let (_, values) = parse_job_variable("LIMIT=1,2,3", TypeKind::Inferred).unwrap();
        assert_eq!(
            values,
            vec![
                Value::Number(Number::Integer(1)),
                Value::Number(Number::Integer(2)),
                Value::Number(Number::Integer(3)),
            ]
        );

        // A value can contain a comma when double-quoted.
        let (_, values) =
            parse_job_variable("HOST=\"a.com,b.com\",c.com", TypeKind::Inferred).unwrap();
        assert_eq!(
            values,
            vec![
                Value::String("a.com,b.com".to_string()),
                Value::String("c.com".to_string()),
            ]
        );

        // An explicit empty string can be used as a value.
        let (_, values) = parse_job_variable("SUFFIX=\"\",x", TypeKind::Inferred).unwrap();
        assert_eq!(
            values,
            vec![Value::String(String::new()), Value::String("x".to_string())]
        );

        // Surrounding spaces are trimmed.
        let (_, values) = parse_job_variable("SUFFIX= red , green ", TypeKind::Inferred).unwrap();
        assert_eq!(
            values,
            vec![
                Value::String("red".to_string()),
                Value::String("green".to_string()),
            ]
        );
    }

    #[test]
    fn test_parse_job_variable_error() {
        assert_eq!(
            parse_job_variable("NAME", TypeKind::Inferred)
                .err()
                .unwrap(),
            CliOptionsError::Error("Missing value for job variable NAME!".to_string())
        );
        assert_eq!(
            parse_job_variable("NAME=", TypeKind::Inferred)
                .err()
                .unwrap(),
            CliOptionsError::Error("Missing value for job variable NAME!".to_string())
        );
        assert_eq!(
            parse_job_variable("NAME=a,,b", TypeKind::Inferred)
                .err()
                .unwrap(),
            CliOptionsError::Error("Empty value in job variable NAME".to_string())
        );
        assert_eq!(
            parse_job_variable("NAME=a,", TypeKind::Inferred)
                .err()
                .unwrap(),
            CliOptionsError::Error("Empty value in job variable NAME".to_string())
        );
        assert_eq!(
            parse_job_variable("newUuid=red,green", TypeKind::Inferred)
                .err()
                .unwrap(),
            CliOptionsError::Error(
                "Variable newUuid conflicts with the newUuid function, use a different name."
                    .to_string()
            )
        );
    }

    #[test]
    fn test_parse_value() {
        assert_eq!(
            parse_value("Jennifer", TypeKind::Inferred).unwrap(),
            Value::String("Jennifer".to_string())
        );
        assert_eq!(
            parse_value("true", TypeKind::Inferred).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            parse_value("30", TypeKind::Inferred).unwrap(),
            Value::Number(Number::Integer(30))
        );
        assert_eq!(
            parse_value("30", TypeKind::String).unwrap(),
            Value::String("30".to_string())
        );
        assert_eq!(
            parse_value("1.7", TypeKind::Inferred).unwrap(),
            Value::Number(Number::Float(1.7))
        );
        assert_eq!(
            parse_value("1.7", TypeKind::String).unwrap(),
            Value::String("1.7".to_string())
        );
        assert_eq!(
            parse_value("1.0", TypeKind::Inferred).unwrap(),
            Value::Number(Number::Float(1.0))
        );
        assert_eq!(
            parse_value("-1.0", TypeKind::Inferred).unwrap(),
            Value::Number(Number::Float(-1.0))
        );
        assert_eq!(
            parse_value("\"123\"", TypeKind::Inferred).unwrap(),
            Value::String("123".to_string())
        );
        assert_eq!(
            parse_value("\"123\"", TypeKind::String).unwrap(),
            Value::String("\"123\"".to_string())
        );
        assert_eq!(
            parse_value("null", TypeKind::Inferred).unwrap(),
            Value::Null
        );
    }

    #[test]
    fn test_parse_value_error() {
        assert_eq!(
            parse_value("\"123", TypeKind::Inferred).err().unwrap(),
            CliOptionsError::Error("Value should end with a double quote".to_string())
        );
    }
}
