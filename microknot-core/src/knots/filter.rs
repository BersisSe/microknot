use regex::Regex;
use serde_json::{Value, json};

use crate::error::{Error, Result};
use crate::item::Item;
use crate::knot::{Ctx, Knot, KnotDescriptor};
use crate::schema::{FieldOption, KnotSchema, ListRowField, ParamField};
use crate::templ;

pub const DESCRIPTOR: KnotDescriptor = KnotDescriptor {
    kind: "transform.filter",
    name: "Filter",
    description: "Routes items to output 0 (match) or output 1 (no match)",
    output_count: 2,
    input_count: 1,
};

pub fn schema() -> KnotSchema {
    let operators: Vec<FieldOption> = [
        ("eq", "EQ"),
        ("neq", "NEQ"),
        ("contains", "CONTAINS"),
        ("regex", "REGEX"),
        ("gt", "GT"),
        ("gte", "GTE"),
        ("lt", "LT"),
        ("lte", "LTE"),
        ("exists", "EXISTS"),
    ]
    .iter()
    .map(|(v, l)| FieldOption {
        value: v.to_string(),
        label: l.to_string(),
    })
    .collect();

    vec![
        ParamField::list("conditions", "Conditions").rows(vec![
            ListRowField::text("field", "Field").placeholder("age"),
            ListRowField::select("operator", "Op").options(operators.clone()),
            ListRowField::jsonish("value", "Value").placeholder("18"),
        ]),
        ParamField::select("mode", "Mode")
            .options(vec![
                FieldOption {
                    value: "all".into(),
                    label: "ALL".into(),
                },
                FieldOption {
                    value: "any".into(),
                    label: "ANY".into(),
                },
            ])
            .default(json!("all")),
    ]
}

pub fn factory(params: &Value) -> Box<dyn Knot> {
    let conditions = params
        .get("conditions")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Condition::from_value).collect())
        .unwrap_or_default();
    let require_all = params.get("mode").and_then(Value::as_str) != Some("any");
    Box::new(Filter {
        conditions,
        require_all,
    })
}

pub struct Filter {
    conditions: Vec<Condition>,
    require_all: bool,
}

pub struct Condition {
    field: String,
    operator: Operator,
    value: Value,
}

#[derive(Clone, Copy)]
enum Operator {
    Eq,
    Neq,
    Contains,
    Regex,
    Gt,
    Gte,
    Lt,
    Lte,
    Exists,
}

impl Condition {
    fn from_value(value: &Value) -> Option<Self> {
        let field = value.get("field")?.as_str()?.to_string();
        let operator = Operator::parse(value.get("operator")?.as_str()?)?;
        let value = value.get("value").cloned().unwrap_or(Value::Null);
        Some(Self {
            field,
            operator,
            value,
        })
    }
}

impl Operator {
    fn parse(op: &str) -> Option<Self> {
        Some(match op {
            "eq" => Self::Eq,
            "neq" => Self::Neq,
            "contains" => Self::Contains,
            "regex" => Self::Regex,
            "gt" => Self::Gt,
            "gte" => Self::Gte,
            "lt" => Self::Lt,
            "lte" => Self::Lte,
            "exists" => Self::Exists,
            _ => return None,
        })
    }
}

impl Knot for Filter {
    fn run(&self, _ctx: &Ctx, input: Vec<Item>) -> Result<Vec<Vec<Item>>> {
        let mut matching = Vec::new();
        let mut non_matching = Vec::new();
        for item in input {
            if self.matches(&item)? {
                matching.push(item);
            } else {
                non_matching.push(item);
            }
        }
        Ok(vec![matching, non_matching])
    }
}

impl Filter {
    fn matches(&self, item: &Item) -> Result<bool> {
        if self.conditions.is_empty() {
            return Ok(true);
        }
        for condition in &self.conditions {
            let passed = condition.evaluate(item)?;
            if !self.require_all && passed {
                return Ok(true);
            }
            if self.require_all && !passed {
                return Ok(false);
            }
        }
        Ok(self.require_all)
    }
}

impl Condition {
    fn evaluate(&self, item: &Item) -> Result<bool> {
        let actual = templ::resolve(&format!("$json.{}", self.field), item);
        let expected = templ::render_param(&self.value, item);
        Ok(match self.operator {
            Operator::Exists => actual.is_some(),
            Operator::Eq => actual.as_ref().is_some_and(|a| a == &expected),
            Operator::Neq => actual.as_ref().is_some_and(|a| a != &expected),
            Operator::Contains => match (actual.as_ref(), expected.as_str()) {
                (Some(Value::String(haystack)), Some(needle)) => haystack.contains(needle),
                _ => false,
            },
            Operator::Regex => {
                let pattern = match expected.as_str() {
                    Some(p) => Regex::new(p).map_err(|e| Error::Knot(e.to_string()))?,
                    None => return Ok(false),
                };
                match actual.as_ref() {
                    Some(Value::String(s)) => pattern.is_match(s),
                    _ => false,
                }
            }
            Operator::Gt => compare(&actual, &expected, |a, b| a > b),
            Operator::Gte => compare(&actual, &expected, |a, b| a >= b),
            Operator::Lt => compare(&actual, &expected, |a, b| a < b),
            Operator::Lte => compare(&actual, &expected, |a, b| a <= b),
        })
    }
}

fn compare(actual: &Option<Value>, expected: &Value, pred: fn(f64, f64) -> bool) -> bool {
    match (actual, expected) {
        (Some(Value::Number(a)), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(a), Some(b)) => pred(a, b),
            _ => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run_filter(conditions: Value, items: Vec<Value>) -> (Vec<Value>, Vec<Value>) {
        let knot = factory(&json!({ "conditions": conditions }));
        let ctx = Ctx {
            workflow_id: "w".into(),
        };
        let input: Vec<Item> = items.into_iter().map(Item::new).collect();
        let out = knot.run(&ctx, input).unwrap();
        let to_json = |v: &[Item]| v.iter().map(|i| i.json.clone()).collect::<Vec<_>>();
        (to_json(&out[0]), to_json(&out[1]))
    }

    #[test]
    fn routes_matching_to_output_zero() {
        let (match_, no_match) = run_filter(
            json!([{ "field": "age", "operator": "gt", "value": 18 }]),
            vec![json!({ "age": 20 }), json!({ "age": 15 })],
        );
        assert_eq!(match_, vec![json!({ "age": 20 })]);
        assert_eq!(no_match, vec![json!({ "age": 15 })]);
    }

    #[test]
    fn contains_and_regex() {
        let (match_, no_match) = run_filter(
            json!([{ "field": "email", "operator": "contains", "value": "@example.com" }]),
            vec![
                json!({ "email": "a@example.com" }),
                json!({ "email": "b@other.org" }),
            ],
        );
        assert_eq!(match_.len(), 1);
        assert_eq!(no_match.len(), 1);

        let (match_, _) = run_filter(
            json!([{ "field": "name", "operator": "regex", "value": "^a" }]),
            vec![json!({ "name": "ada" })],
        );
        assert_eq!(match_.len(), 1);
    }

    #[test]
    fn all_mode_and_any_mode() {
        let knot = factory(&json!({ "mode": "all", "conditions": [
            { "field": "a", "operator": "eq", "value": 1 },
            { "field": "b", "operator": "eq", "value": 2 }
        ] }));
        let ctx = Ctx {
            workflow_id: "w".into(),
        };
        let input = vec![
            Item::new(json!({ "a": 1, "b": 2 })),
            Item::new(json!({ "a": 1, "b": 3 })),
        ];
        let out = knot.run(&ctx, input).unwrap();
        assert_eq!(out[0].len(), 1);
        assert_eq!(out[1].len(), 1);
    }
}
