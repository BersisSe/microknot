use std::collections::{HashMap, HashSet};

use serde::Serialize;
use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::item::Item;
use crate::knot::{Ctx, Registry};
use crate::model::{Knot, Workflow};

#[derive(Debug, Clone, Serialize)]
pub struct RunStep {
    pub knot: String,
    pub kind: String,
    pub status: String,
    pub items: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunResult {
    pub status: String,
    pub steps: Vec<RunStep>,
}

pub fn run_workflow(workflow: &Workflow, registry: &Registry, body: Option<Value>) -> Result<RunResult> {
    let order = topo_order(workflow)?;
    let by_id: HashMap<&str, &Knot> = workflow.knots.iter().map(|k| (k.id.as_str(), k)).collect();

    let mut streams: HashMap<String, Vec<Item>> = HashMap::new();
    let seed = body.unwrap_or(json!({}));
    let mut steps = Vec::new();

    for knot_id in order {
        let knot = by_id[knot_id.as_str()];
        let input: Vec<Item> = match knot.kind.as_str() {
            kind if kind.starts_with("trigger.") => {
                let stream = streams.remove(&knot.id);
                if kind == "trigger.webhook" {
                    vec![Item::new(seed.clone())]
                } else {
                    stream.unwrap_or_else(|| vec![Item::new(seed.clone())])
                }
            }
            _ => streams.remove(&knot.id).unwrap_or_else(|| vec![Item::new(seed.clone())]),
        };

        let ctx = Ctx {
            workflow_id: workflow.id.clone(),
        };
        let instance = registry
            .create(&knot.kind, &knot.params)
            .ok_or_else(|| Error::InvalidWorkflow(format!("unknown knot type `{}`", knot.kind)))?;

        let outputs = instance.run(&ctx, input).map_err(|e| {
            Error::Knot(format!("knot `{}` ({}) failed: {}", knot.id, knot.kind, e))
        })?;

        for (output_index, items) in outputs.iter().enumerate() {
            for conn in workflow.connections.iter().filter(|c| {
                c.from == knot.id && c.from_output == output_index
            }) {
                let entry = streams.entry(conn.to.clone()).or_default();
                entry.extend(items.clone());
            }
        }

        steps.push(RunStep {
            knot: knot.id.clone(),
            kind: knot.kind.clone(),
            status: "ok".into(),
            items: outputs.iter().map(|o| o.len()).sum(),
        });
    }

    Ok(RunResult {
        status: "ok".into(),
        steps,
    })
}

fn topo_order(workflow: &Workflow) -> Result<Vec<String>> {
    let by_id: HashSet<&str> = workflow.knots.iter().map(|k| k.id.as_str()).collect();
    let mut indegree: HashMap<&str, usize> = workflow
        .knots
        .iter()
        .map(|k| (k.id.as_str(), 0))
        .collect();
    let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();

    for conn in &workflow.connections {
        if !by_id.contains(conn.from.as_str()) || !by_id.contains(conn.to.as_str()) {
            return Err(Error::InvalidWorkflow(format!(
                "connection references unknown knot: `{}` -> `{}`",
                conn.from, conn.to
            )));
        }
        if let Some(count) = indegree.get_mut(conn.to.as_str()) {
            *count += 1;
        }
        outgoing.entry(conn.from.as_str()).or_default().push(conn.to.as_str());
    }

    let mut queue: Vec<&str> = indegree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(k, _)| *k)
        .collect();
    queue.sort();
    let mut order = Vec::with_capacity(workflow.knots.len());

    let mut index = 0;
    while index < queue.len() {
        let current = queue[index];
        index += 1;
        order.push(current.to_string());
        if let Some(nexts) = outgoing.get(current) {
            for next in nexts {
                if let Some(count) = indegree.get_mut(next) {
                    *count -= 1;
                    if *count == 0 {
                        queue.push(next);
                    }
                }
            }
        }
    }

    if order.len() != workflow.knots.len() {
        return Err(Error::InvalidWorkflow("workflow contains a cycle".into()));
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Connection;

    fn workflow(knots: Vec<(String, String)>) -> Workflow {
        Workflow {
            id: "wf".into(),
            name: "test".into(),
            knots: knots
                .into_iter()
                .map(|(id, kind)| Knot::new(id, kind, json!({})))
                .collect(),
            connections: vec![Connection {
                from: "a".into(),
                from_output: 0,
                to: "b".into(),
                to_input: 0,
            }],
            active: false,
        }
    }

    #[test]
    fn runs_linear_workflow() {
        let wf = workflow(vec![("a".into(), "notify.log".into()), ("b".into(), "notify.log".into())]);
        let registry = Registry::default();
        let result = run_workflow(&wf, &registry, None).unwrap();
        assert_eq!(result.status, "ok");
        assert_eq!(result.steps.len(), 2);
    }

    #[test]
    fn fails_on_unknown_knot() {
        let wf = Workflow {
            id: "wf".into(),
            name: "test".into(),
            knots: vec![Knot::new("a", "nope.nope", json!({}))],
            connections: vec![],
            active: false,
        };
        let registry = Registry::default();
        let err = run_workflow(&wf, &registry, None).unwrap_err();
        assert!(err.to_string().contains("unknown knot type"));
    }

    #[test]
    fn detects_cycles() {
        let wf = Workflow {
            id: "wf".into(),
            name: "cycle".into(),
            knots: vec![Knot::new("a", "notify.log", json!({})), Knot::new("b", "notify.log", json!({}))],
            connections: vec![
                Connection { from: "a".into(), from_output: 0, to: "b".into(), to_input: 0 },
                Connection { from: "b".into(), from_output: 0, to: "a".into(), to_input: 0 },
            ],
            active: false,
        };
        let registry = Registry::default();
        let err = run_workflow(&wf, &registry, None).unwrap_err();
        assert!(err.to_string().contains("cycle"));
    }

    #[test]
    fn schedule_trigger_errors_on_manual_run() {
let wf = workflow(vec![("a".into(), "trigger.schedule".into()), ("b".into(), 
"notify.log".into())]);
        let registry = Registry::default();
        let err = run_workflow(&wf, &registry, None).unwrap_err();
        assert!(err.to_string().contains("schedule trigger cannot be executed manually"));
    }
}
