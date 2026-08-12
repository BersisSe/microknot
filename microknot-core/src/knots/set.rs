use serde_json::Value;

use crate::error::Result;
use crate::item::Item;
use crate::knot::{Ctx, Knot, KnotDescriptor};
use crate::schema::{KnotSchema, ListRowField, ParamField};
use crate::templ;

pub const DESCRIPTOR: KnotDescriptor = KnotDescriptor {
    kind: "transform.set",
    name: "Set",
    description: "Sets or modifies JSON fields on each item",
    output_count: 1,
    input_count: 1,
};

pub fn schema() -> KnotSchema {
    vec![ParamField::list("assignments", "Assignments").rows(vec![
        ListRowField::text("field", "Field").placeholder("user.name"),
        ListRowField::jsonish("value", "Value")
            .placeholder("value or {{ $json.field }}"),
    ])]
}

pub fn factory(params: &Value) -> Box<dyn Knot> {
    let assignments = params
        .get("assignments")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let field = a.get("field")?.as_str()?.to_string();
                    let value = a.get("value")?.clone();
                    Some((field, value))
                })
                .collect()
        })
        .unwrap_or_default();
    Box::new(Set { assignments })
}

pub struct Set {
    assignments: Vec<(String, Value)>,
}

impl Knot for Set {
    fn run(&self, _ctx: &Ctx, input: Vec<Item>) -> Result<Vec<Vec<Item>>> {
        let mut out = Vec::with_capacity(input.len());
        for item in input {
            let mut json = item.json;
            for (field, value) in &self.assignments {
                let rendered = templ::render_param(value, &Item::new(json.clone()));
                set_path(&mut json, field, rendered);
            }
            out.push(Item::new(json));
        }
        Ok(vec![out])
    }
}

fn set_path(root: &mut Value, path: &str, value: Value) {
    let parts: Vec<&str> = path.split('.').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return;
    }
    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        if !current.is_object() {
            *current = Value::Object(serde_json::Map::new());
        }
        let map = current.as_object_mut().expect("object after ensure");
        if !map.contains_key(*part) {
            map.insert((*part).to_string(), Value::Object(serde_json::Map::new()));
        }
        current = map.get_mut(*part).expect("inserted key");
    }
    let last = parts.last().expect("non-empty");
    if let Value::Object(map) = current {
        map.insert((*last).to_string(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sets_flat_and_nested_fields() {
        let knot = factory(&json!({ "assignments": [
            { "field": "title", "value": "Hello {{ $json.user }}" },
            { "field": "meta.count", "value": "{{ $json.n }}" }
        ] }));
        let items = vec![Item::new(json!({ "user": "ada", "n": 5 }))];
        let ctx = Ctx { workflow_id: "w".into() };
        let out = knot.run(&ctx, items).unwrap();
        assert_eq!(out[0][0].json, json!({ "user": "ada", "n": 5, "title": "Hello ada", "meta": { "count": 5 } }));
    }
}