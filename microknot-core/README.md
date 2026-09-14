# microknot-core

The workflow engine behind [Microknot](https://github.com/BersisSe/microknot), packaged as a library so you can embed it in your own binary.

Workflows are JSON documents: a set of knots (nodes) connected by typed ports, executed in topological order. Each knot receives a list of JSON items, processes them, and passes the results downstream.

## Usage

```rust
use microknot_core::{Registry, Store, Workflow, run_workflow};
use serde_json::json;

// From JSON (same format the REST API accepts) ...
let wf: Workflow = serde_json::from_str(r#"{
    "name": "demo",
    "knots": [
        { "id": "a", "type": "trigger.webhook", "params": {} },
        { "id": "b", "type": "notify.log", "params": { "message": "hi {{ $json.name }}" } }
    ],
    "connections": [{ "from": "a", "fromOutput": 0, "to": "b", "toInput": 0 }]
}"#)?;

// ... or from the builder.
let wf = WorkflowBuilder::new("demo")
    .knot("a", "trigger.webhook", json!({}))
    .knot("b", "notify.log", json!({ "message": "hi {{ $json.name }}" }))
    .connect("a", "b")
    .build()?;

wf.validate()?;

// Persist (SQLite, WAL mode) if you want to.
let store = Store::open("microknot.db")?;
store.insert_workflow(&wf)?;

// Run it. The seed item becomes the input of the trigger knot.
let registry = Registry::default();
let result = run_workflow(&wf, &registry, Some(json!({ "name": "ada" })))?;
assert_eq!(result.status, "ok");
```

## Bundled knots

`trigger.webhook`, `trigger.schedule`, `http.request`, `transform.filter`, `transform.set`, `transform.delay`, `ai.call`, `notify.log`, `notify.resend`.

## Writing your own knot

Implement the `Knot` trait and register it in a `Registry`. `run` receives the input items and returns one output stream per output port:

```rust
impl Knot for UpperCase {
    fn run(&self, _ctx: &Ctx, items: Vec<Item>) -> Result<Vec<Vec<Item>>> {
        Ok(vec![items])
    }
}

let mut registry = Registry::default();
registry.register(DESCRIPTOR, factory, schema);
```

See `microknot-core/src/knots/log.rs` for the smallest complete example, and `schema.rs` for how parameter schemas are declared (the dashboard renders its forms from them).

## Status

Pre-1.0. The API may change without notice. Licensed under AGPL-3.0.
