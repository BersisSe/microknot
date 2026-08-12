use serde_json::Value;

use crate::error::Result;
use crate::item::Item;
use crate::knot::{Ctx, Knot, KnotDescriptor};
use crate::schema::{KnotSchema, ParamField};
use crate::templ;

pub const DESCRIPTOR: KnotDescriptor = KnotDescriptor {
    kind: "notify.log",
    name: "Log",
    description: "Logs each item to the console",
    output_count: 1,
    input_count: 1,
};

pub fn schema() -> KnotSchema {
    vec![
        ParamField::textarea("message", "Message")
            .placeholder("Log {{ $json.field }}")
            .help("Blank logs the whole item as JSON."),
    ]
}

pub fn factory(params: &Value) -> Box<dyn Knot> {
    Box::new(Log {
        message: params
            .get("message")
            .and_then(Value::as_str)
            .map(String::from),
    })
}

pub struct Log {
    message: Option<String>,
}

impl Knot for Log {
    fn run(&self, ctx: &Ctx, input: Vec<Item>) -> Result<Vec<Vec<Item>>> {
        for item in &input {
            match &self.message {
                Some(message) => println!(
                    "[{}] {}",
                    ctx.workflow_id,
                    templ::render_string(message, item)
                ),
                None => println!("[{}] {}", ctx.workflow_id, item.json),
            }
        }
        Ok(vec![input])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knot::Ctx;
    use serde_json::json;

    #[test]
    fn passes_items_through() {
        let knot = factory(&json!({}));
        let items = vec![Item::new(json!({ "a": 1 }))];
        let ctx = Ctx {
            workflow_id: "w".into(),
        };
        let out = knot.run(&ctx, items).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 1);
        assert_eq!(out[0][0].json, json!({ "a": 1 }));
    }
}
