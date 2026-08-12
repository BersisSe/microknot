use serde_json::{Value, json};

use crate::error::{Error, Result};
use crate::item::Item;
use crate::knot::{Ctx, Knot, KnotDescriptor};
use crate::schema::KnotSchema;

pub const WEBHOOK: KnotDescriptor = KnotDescriptor {
    kind: "trigger.webhook",
    name: "Webhook",
    description: "Starts a workflow from an incoming HTTP request",
    output_count: 1,
    input_count: 0,
};

pub const SCHEDULE: KnotDescriptor = KnotDescriptor {
    kind: "trigger.schedule",
    name: "Schedule",
    description: "Starts a workflow on a schedule",
    input_count: 0,
    output_count: 1,
};

pub fn webhook_schema() -> KnotSchema {
    Vec::new()
}

pub fn schedule_schema() -> KnotSchema {
    Vec::new()
}

pub fn webhook_factory(_params: &Value) -> Box<dyn Knot> {
    Box::new(Webhook)
}

pub fn schedule_factory(_params: &Value) -> Box<dyn Knot> {
    Box::new(Schedule)
}

pub struct Webhook;

impl Knot for Webhook {
    fn run(&self, _ctx: &Ctx, input: Vec<Item>) -> Result<Vec<Vec<Item>>> {
        // Triggers receive items produced upstream (e.g. an inbound request body).
        let output = if input.is_empty() {
            vec![Item::new(json!({}))]
        } else {
            input
        };
        Ok(vec![output])
    }
}

pub struct Schedule;

impl Knot for Schedule {
    fn run(&self, ctx: &Ctx, _input: Vec<Item>) -> Result<Vec<Vec<Item>>> {
        Err(Error::Knot(format!(
            "schedule trigger cannot be executed manually (workflow `{}`)",
            ctx.workflow_id
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn webhook_seeds_empty_item() {
        let ctx = Ctx {
            workflow_id: "w".into(),
        };
        let out = Webhook.run(&ctx, vec![]).unwrap();
        assert_eq!(out[0][0].json, json!({}));
    }
}
