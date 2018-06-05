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

impl Default for Type {
    fn default() -> Self {
        Type::null
    }
}

#[derive(Deserialize)]
pub struct SourceTypeInfo {
    #[serde(rename = "type")]
    #[serde(default)]
    type_: Type,
    #[serde(default)]
    properties: IndexMap<String, Box<SourceTypeInfo>>,
    items: Option<Box<SourceTypeInfo>>,
    #[serde(rename = "$ref")]
    #[serde(default)]
    ref_: String,
    #[serde(default)]
    definitions: IndexMap<String, Box<SourceTypeInfo>>,
}

pub struct Schema {
    types: Vec<TypeInfo>,
    root: usize,
}

impl Schema {
    pub fn root(&self) -> &TypeInfo {
        &self.types[self.root]
    }

    pub fn type_info(&self, idx: usize) -> &TypeInfo {
        &self.types[idx]
    }
}

#[derive(Default)]
pub struct TypeInfo {
    type_: Type,
    properties: IndexMap<String, usize>,
    items: usize,
}

#[derive(Default)]
struct Translator {
    resolved: IndexMap<String, usize>,
    types: Vec<TypeInfo>,
}

impl Translator {
    fn translate(
        &mut self,
        source: &SourceTypeInfo,
        defs: &IndexMap<String, Box<SourceTypeInfo>>,
    ) -> TypeInfo {
        let items = if let Some(ref items) = source.items {
            self.resolve(items, defs)
        } else {
            use std::usize;
            usize::MAX
        };

        let properties = source
            .properties
            .iter()
            .map(|(k, v)| (k.clone(), self.resolve(v, defs)))
            .collect::<IndexMap<_, _>>();
        TypeInfo {
            type_: source.type_,
            properties,
            items,
        }
    }

    fn resolve(
        &mut self,
        source: &SourceTypeInfo,
        defs: &IndexMap<String, Box<SourceTypeInfo>>,
    ) -> usize {
        if source.ref_.is_empty() {
            let t = self.translate(source, defs);
            self.types.push(t);
            self.types.len() - 1
        } else {
            if let Some(idx) = self.resolved.get(&source.ref_).cloned() {
                idx
            } else {
                let idx = self.types.len();
                // Put a placeholder first, so we can record its number in the resolved map
                self.types.push(Default::default());
                self.resolved.insert(source.ref_.clone(), idx);

                assert!(source.ref_.starts_with("#/definitions/"));
                let def = &defs[&source.ref_["#/definitions/".len()..]];
                self.types[idx] = self.translate(def, defs);
                idx
            }
        }
    }
}

fn translate(source: &SourceTypeInfo) -> Schema {
    let mut tx: Translator = Default::default();
    let root = tx.resolve(source, &source.definitions);
    Schema {
        types: tx.types,
        root,
    }
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

fn validate(schema: &Schema, value: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    validate_inner(
        schema,
        schema.root(),
        value,
        &mut String::new(),
        &mut errors,
    );
    errors
}

fn validate_inner(
    schema: &Schema,
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
            let child_info = schema.type_info(*child_info);
            validate_inner(schema, child_info, &value[key], path, errors);
            path.truncate(len);
        }
    } else if type_info.type_ == Type::array {
        let child_info = schema.type_info(type_info.items);
        for (idx, child_value) in value.as_array().unwrap().iter().enumerate() {
            let len = path.len();
            write!(path, "/{}", idx).unwrap();
            validate_inner(schema, child_info, &child_value, path, errors);
            path.truncate(len);
        }
    }
}

fn run_test(schema: &Schema, value: &Value, expected: &[&str]) {
    let errs = validate(schema, value);
    let errs = errs.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    assert_eq!(errs.as_slice(), expected);
}

#[test]
pub fn test() {
    let basic = include_str!("cyclic.schema.json");
    let schema: SourceTypeInfo = serde_json::from_str(basic).unwrap();

    let schema = translate(&schema);

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

    // Recursive!
    let val3 = json!({
      "firstName": [{}, "Alex"],
      "age": [],
      "children": [{
        "firstName": ["Alice"],
        "lastName": 12,
      }]
    });
    run_test(
        &schema,
        &val3,
        &[
            "/firstName/0: type mismatch, expected: string, actual: object",
            "/age: type mismatch, expected: number, actual: array",
            "/children/0/lastName: type mismatch, expected: string, actual: number",
        ],
    );
}
