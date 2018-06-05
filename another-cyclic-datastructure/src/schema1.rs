#![allow(non_camel_case_types)]

use indexmap::IndexMap;
use serde_json::{self, Value};
use std::fmt::Write;

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum Type {
    object,
    string,
    number,
    bool,
    array,
    null,
}

#[derive(Deserialize)]
pub struct TypeInfo {
    #[serde(rename = "type")]
    type_: Type,
    #[serde(default)]
    properties: IndexMap<String, Box<TypeInfo>>,
    items: Option<Box<TypeInfo>>,
}

fn type_of(value: &Value) -> Type {
    match *value {
        Value::Number(_) => Type::number,
        Value::String(_) => Type::string,
        Value::Bool(_) => Type::bool,
        Value::Array(_) => Type::array,
        Value::Object(_) => Type::object,
        Value::Null => Type::null,
    }
}

fn validate(type_info: &TypeInfo, value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    validate_inner(type_info, value, &mut String::new(), &mut errors);
    errors
}

fn validate_inner(
    type_info: &TypeInfo,
    value: &Value,
    path: &mut String,
    errors: &mut Vec<String>,
) {
    // Treat null as always valid!
    if value.is_null() {
        return;
    }

    let actual_type = type_of(value);
    if actual_type != type_info.type_ {
        errors.push(format!(
            "{}: type mismatch, expected: {:?}, actual: {:?}",
            if path.is_empty() {
                "/"
            } else {
                path.as_str()
            },
            type_info.type_,
            actual_type
        ));
        return;
    }

    if type_info.type_ == Type::object {
        for (key, child_info) in &type_info.properties {
            let len = path.len();
            write!(path, "/{}", key).unwrap();
            validate_inner(child_info, &value[key], path, errors);
            path.truncate(len);
        }
    } else if type_info.type_ == Type::array {
        let child_info = type_info.items.as_ref().unwrap();
        for (idx, child_value) in value.as_array().unwrap().iter().enumerate() {
            let len = path.len();
            write!(path, "/{}", idx).unwrap();
            validate_inner(&child_info, &child_value, path, errors);
            path.truncate(len);
        }
    }
}

fn run_test(schema: &TypeInfo, value: &Value, expected: &[&str]) {
    let errs = validate(schema, value);
    let errs = errs.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    assert_eq!(errs.as_slice(), expected);
}

#[test]
pub fn test() {
    let basic = include_str!("basic.schema.json");
    let schema = serde_json::from_str(basic).unwrap();

    let val1 = json!("hello");
    run_test(
        &schema,
        &val1,
        &["/: type mismatch, expected: object, actual: string"],
    );

    let val2 = json!({
      "firstName": [{}, "Alex"],
      "age": []
    });
    run_test(
        &schema,
        &val2,
        &[
            "/firstName/0: type mismatch, expected: string, actual: object",
            "/age: type mismatch, expected: number, actual: array",
        ],
    );
}
