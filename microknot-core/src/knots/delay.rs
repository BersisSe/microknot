use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use crate::error::Result;
use crate::item::Item;
use crate::knot::{Ctx, Knot, KnotDescriptor};
use crate::schema::{KnotSchema, ParamField};

pub const DESCRIPTOR: KnotDescriptor = KnotDescriptor {
    kind: "transform.delay",
    name: "Delay",
    description: "Waits for a number of milliseconds before continuing",
    output_count: 1,
    input_count: 1,
};

pub fn schema() -> KnotSchema {
    vec![ParamField::number("milliseconds", "Milliseconds").default(json!(0))]
}

pub fn factory(params: &Value) -> Box<dyn Knot> {
    let milliseconds = params
        .get("milliseconds")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    Box::new(Delay { milliseconds })
}

pub struct Delay {
    milliseconds: u64,
}

impl Knot for Delay {
    fn run(&self, _ctx: &Ctx, input: Vec<Item>) -> Result<Vec<Vec<Item>>> {
        thread::sleep(Duration::from_millis(self.milliseconds));
        Ok(vec![input])
    }
}
