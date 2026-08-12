use serde_json::Value;

use crate::model::{Connection, Knot, Workflow};
use crate::validate;

pub struct WorkflowBuilder {
    id: String,
    name: String,
    knots: Vec<Knot>,
    connections: Vec<Connection>,
    active: bool,
}

impl WorkflowBuilder {
    pub(crate) fn new(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            knots: Vec::new(),
            connections: Vec::new(),
            active: false,
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn knot(mut self, id: impl Into<String>, kind: impl Into<String>, params: Value) -> Self {
        self.knots.push(Knot::new(id, kind, params));
        self
    }

    pub fn connect(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.connections.push(Connection {
            from: from.into(),
            from_output: 0,
            to: to.into(),
            to_input: 0,
        });
        self
    }

    pub fn connect_output(
        mut self,
        from: impl Into<String>,
        from_output: usize,
        to: impl Into<String>,
        to_input: usize,
    ) -> Self {
        self.connections.push(Connection {
            from: from.into(),
            from_output,
            to: to.into(),
            to_input,
        });
        self
    }

    pub fn build(self) -> std::result::Result<Workflow, Vec<validate::ValidationError>> {
        let wf = Workflow {
            id: self.id,
            name: self.name,
            knots: self.knots,
            connections: self.connections,
            active: self.active,
        };
        validate::validate(&wf)?;
        Ok(wf)
    }
}

impl Workflow {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(name: impl Into<String>) -> WorkflowBuilder {
        WorkflowBuilder::new(name.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_valid_workflow() {
        let wf = Workflow::new("Test")
            .knot(
                "a",
                "trigger.schedule",
                serde_json::json!({ "cron": "* * * * *" }),
            )
            .knot(
                "b",
                "http.request",
                serde_json::json!({ "url": "https://x.dev" }),
            )
            .connect("a", "b")
            .active(true)
            .build()
            .unwrap();

        assert_eq!(wf.name, "Test");
        assert!(wf.active);
        assert_eq!(wf.knots.len(), 2);
        assert_eq!(wf.connections.len(), 1);
        wf.validate().unwrap();
    }

    #[test]
    fn build_rejects_invalid() {
        let err = Workflow::new("Bad")
            .knot("a", "http.request", serde_json::json!({}))
            .connect("a", "missing")
            .build();
        assert!(err.is_err());
        assert_eq!(err.unwrap_err().len(), 1);
    }

    #[test]
    fn explicit_id_and_outputs() {
        let wf = Workflow::new("T")
            .id("custom")
            .knot("a", "transform.filter", serde_json::json!({}))
            .knot("b", "notify.log", serde_json::json!({}))
            .connect_output("a", 1, "b", 0)
            .build()
            .unwrap();
        assert_eq!(wf.id, "custom");
        assert_eq!(wf.connections[0].from_output, 1);
    }
}
