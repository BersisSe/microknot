use std::collections::{HashMap, HashSet, VecDeque};

use crate::model::Workflow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub message: String,
}

impl ValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Workflow {
    pub fn validate(&self) -> std::result::Result<(), Vec<ValidationError>> {
        validate(self)
    }
}

pub fn validate(wf: &Workflow) -> std::result::Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    if wf.name.trim().is_empty() {
        errors.push(ValidationError::new("workflow name must not be empty"));
    }

    if wf.knots.is_empty() {
        errors.push(ValidationError::new(
            "workflow must contain at least one knot",
        ));
    }

    let mut seen: HashSet<&str> = HashSet::new();
    for knot in &wf.knots {
        if knot.id.trim().is_empty() {
            errors.push(ValidationError::new("knot id must not be empty"));
            continue;
        }
        if !seen.insert(knot.id.as_str()) {
            errors.push(ValidationError::new(format!(
                "duplicate knot id '{}'",
                knot.id
            )));
        }
    }

    let ids: HashSet<&str> = wf.knots.iter().map(|k| k.id.as_str()).collect();
    let mut connections: HashSet<(String, usize, String, usize)> = HashSet::new();
    for c in &wf.connections {
        if !ids.contains(c.from.as_str()) {
            errors.push(ValidationError::new(format!(
                "connection references unknown knot '{}' (from)",
                c.from
            )));
        }
        if !ids.contains(c.to.as_str()) {
            errors.push(ValidationError::new(format!(
                "connection references unknown knot '{}' (to)",
                c.to
            )));
        }
        if !connections.insert((c.from.clone(), c.from_output, c.to.clone(), c.to_input)) {
            errors.push(ValidationError::new(format!(
                "duplicate connection {} (output {}) -> {} (input {})",
                c.from, c.from_output, c.to, c.to_input
            )));
        }
    }

    if let Some(cycle) = find_cycle(wf) {
        errors.push(ValidationError::new(format!(
            "cycle detected involving knots: {}",
            cycle.join(", ")
        )));
    }

    validate_registry(wf, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_registry(wf: &Workflow, errors: &mut Vec<ValidationError>) {
    let registry = crate::Registry::default();
    let by_id: HashMap<&str, &crate::model::Knot> =
        wf.knots.iter().map(|k| (k.id.as_str(), k)).collect();

    for knot in &wf.knots {
        if registry.descriptor(&knot.kind).is_none() {
            errors.push(ValidationError::new(format!(
                "unknown knot type '{}'",
                knot.kind
            )));
        }
    }

    for c in &wf.connections {
        let Some(from) = by_id.get(c.from.as_str()) else {
            continue;
        };
        let Some(descriptor) = registry.descriptor(&from.kind) else {
            continue;
        };
        if c.from_output >= descriptor.output_count {
            errors.push(ValidationError::new(format!(
                "knot '{}' (type {}) has no output {}, only {} output(s)",
                c.from, from.kind, c.from_output, descriptor.output_count
            )));
        }
    }
}

fn find_cycle(wf: &Workflow) -> Option<Vec<String>> {
    let order: Vec<&str> = wf.knots.iter().map(|k| k.id.as_str()).collect();
    let index: HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();
    let n = order.len();

    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut in_degree = vec![0usize; n];
    for c in &wf.connections {
        if let (Some(&from), Some(&to)) = (index.get(c.from.as_str()), index.get(c.to.as_str()))
            && from != to
        {
            adjacency[from].push(to);
            in_degree[to] += 1;
        }
    }

    let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut removed = 0usize;
    while let Some(u) = queue.pop_front() {
        removed += 1;
        for &v in &adjacency[u] {
            in_degree[v] -= 1;
            if in_degree[v] == 0 {
                queue.push_back(v);
            }
        }
    }

    let mut cycle: Vec<String> = wf
        .knots
        .iter()
        .filter(|k| in_degree[index[k.id.as_str()]] > 0)
        .map(|k| k.id.clone())
        .collect();

    for c in &wf.connections {
        if c.from == c.to && !cycle.contains(&c.from) {
            cycle.push(c.from.clone());
        }
    }

    if removed == n && cycle.is_empty() {
        None
    } else {
        Some(cycle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Knot;
    use serde_json::json;

    fn workflow(knots: Vec<(&str, &str)>, connections: Vec<(String, usize, String, usize)>) -> Workflow {
        let knots = knots
            .into_iter()
            .map(|(id, kind)| Knot::new(id, kind, json!({})))
            .collect();
        let connections = connections
            .into_iter()
            .map(|(from, from_output, to, to_input)| crate::model::Connection {
                from,
                from_output,
                to,
                to_input,
            })
            .collect();
        Workflow {
            id: "wf".into(),
            name: "Test".into(),
            knots,
            connections,
            active: false,
        }
    }

    fn workflow_simple(knots: Vec<(&str, &str)>, edges: &[(&str, &str)]) -> Workflow {
        workflow(
            knots,
            edges
                .iter()
                .map(|(f, t)| (f.to_string(), 0, t.to_string(), 0))
                .collect(),
        )
    }

    fn errors(wf: &Workflow) -> Vec<String> {
        validate(wf)
            .err()
            .map(|v| v.into_iter().map(|e| e.message).collect())
            .unwrap_or_default()
    }

    #[test]
    fn valid_workflow() {
        let wf = workflow_simple(vec![("a", "trigger.schedule"), ("b", "http.request")], &[("a", "b")]);
        assert!(validate(&wf).is_ok());
    }

    #[test]
    fn empty_name() {
        let mut wf = workflow_simple(vec![("a", "trigger.schedule")], &[]);
        wf.name = "   ".into();
        let msgs = errors(&wf);
        assert!(msgs.iter().any(|m| m.contains("name")));
    }

    #[test]
    fn no_knots() {
        let wf = workflow_simple(vec![], &[]);
        assert!(errors(&wf).iter().any(|m| m.contains("at least one knot")));
    }

    #[test]
    fn duplicate_knot_id() {
        let wf = workflow_simple(vec![("a", "x"), ("a", "y")], &[]);
        assert!(errors(&wf).iter().any(|m| m.contains("duplicate knot id 'a'")));
    }

    #[test]
    fn dangling_connection() {
        let wf = workflow_simple(vec![("a", "x")], &[("a", "missing")]);
        let msgs = errors(&wf);
        assert!(msgs.iter().any(|m| m.contains("unknown knot 'missing'")));
    }

    #[test]
    fn duplicate_connection() {
        let wf = workflow_simple(vec![("a", "x"), ("b", "y")], &[("a", "b"), ("a", "b")]);
        assert!(errors(&wf).iter().any(|m| m.contains("duplicate connection")));
    }

    #[test]
    fn cycle_detected() {
        let wf = workflow_simple(vec![("a", "x"), ("b", "y"), ("c", "z")], &[("a", "b"), ("b", "c"), ("c", "a")]);
        let msgs = errors(&wf);
        assert!(msgs.iter().any(|m| m.contains("cycle")));
    }

    #[test]
    fn self_loop_detected() {
        let wf = workflow_simple(vec![("a", "x")], &[("a", "a")]);
        assert!(errors(&wf).iter().any(|m| m.contains("cycle")));
    }

    #[test]
    fn diamond_is_valid() {
        let wf = workflow_simple(
            vec![("a", "notify.log"), ("b", "transform.set"), ("c", "transform.filter"), ("d", "transform.delay")],
            &[("a", "b"), ("a", "c"), ("b", "d"), ("c", "d")],
        );
        assert!(validate(&wf).is_ok());
    }
}