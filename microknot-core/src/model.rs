use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    #[serde(default = "default_id")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub knots: Vec<Knot>,
    #[serde(default)]
    pub connections: Vec<Connection>,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Knot {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default = "default_params")]
    pub params: Value,
    #[serde(default)]
    pub position: (f32, f32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub from: String,
    #[serde(default)]
    pub from_output: usize,
    pub to: String,
    #[serde(default)]
    pub to_input: usize,
}

fn default_params() -> Value {
    Value::Object(serde_json::Map::new())
}

fn default_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl Knot {
    pub fn new(id: impl Into<String>, kind: impl Into<String>, params: Value) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            name: id,
            kind: kind.into(),
            params,
            position: (0.0, 0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_round_trip() {
        let wf = Workflow {
            id: "wf_1".into(),
            name: "Example".into(),
            knots: vec![Knot {
                id: "n1".into(),
                name: "Request".into(),
                kind: "http.request".into(),
                params: serde_json::json!({ "method": "GET", "url": "https://x.dev" }),
                position: (10.0, 20.0),
            }],
            connections: vec![Connection {
                from: "n1".into(),
                from_output: 1,
                to: "n2".into(),
                to_input: 2,
            }],
            active: true,
        };

        let json = serde_json::to_string(&wf).unwrap();
        let back: Workflow = serde_json::from_str(&json).unwrap();

        assert_eq!(wf, back);
        assert!(json.contains("\"fromOutput\":1"));
        assert!(json.contains("\"toInput\":2"));
        assert!(json.contains("\"type\":\"http.request\""));
        assert!(json.contains("\"knots\""));
    }

    #[test]
    fn deserialize_with_defaults() {
        let wf: Workflow = serde_json::from_str(r#"{ "id": "w", "name": "Min" }"#).unwrap();
        assert!(wf.knots.is_empty());
        assert!(wf.connections.is_empty());
        assert!(!wf.active);
    }

    #[test]
    fn missing_id_is_generated() {
        let wf: Workflow = serde_json::from_str(r#"{ "name": "No id" }"#).unwrap();
        assert!(!wf.id.is_empty());
        assert_eq!(wf.name, "No id");
    }

    #[test]
    fn knot_name_defaults_to_empty() {
        let wf: Workflow =
            serde_json::from_str(r#"{ "name": "w", "knots": [ { "id": "a", "type": "x" } ] }"#)
                .unwrap();
        assert_eq!(wf.knots[0].name, "");
        assert_eq!(wf.knots[0].kind, "x");
    }

    #[test]
    fn knot_new_sets_defaults() {
        let k = Knot::new("a", "trigger.webhook", serde_json::json!({}));
        assert_eq!(k.name, "a");
        assert_eq!(k.position, (0.0, 0.0));
        assert_eq!(k.params, serde_json::json!({}));
    }
}
