use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Declarative param schema for a knot. This is the server-side source of truth
/// for the inspector form: the frontend renders fields purely from this, so new
/// or script-defined knots can describe their params without touching the UI.
///
/// The JSON shape (camelCase) is a contract with `frontend/src/knotSchema.ts`.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FieldOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ListRowField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<FieldOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParamField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<FieldOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<ListRowField>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kv_key_placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kv_value_placeholder: Option<String>,
}

pub type KnotSchema = Vec<ParamField>;

pub fn options(pairs: &[(&str, &str)]) -> Vec<FieldOption> {
    pairs
        .iter()
        .map(|(value, label)| FieldOption {
            value: value.to_string(),
            label: label.to_string(),
        })
        .collect()
}

impl ParamField {
    pub fn text(key: &str, label: &str) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            field_type: "text".into(),
            required: None,
            placeholder: None,
            help: None,
            secret: None,
            default: None,
            options: None,
            rows: None,
            kv_key_placeholder: None,
            kv_value_placeholder: None,
        }
    }

    pub fn number(key: &str, label: &str) -> Self {
        Self::text(key, label).with_type("number")
    }

    pub fn select(key: &str, label: &str) -> Self {
        Self::text(key, label).with_type("select")
    }

    pub fn textarea(key: &str, label: &str) -> Self {
        Self::text(key, label).with_type("textarea")
    }

    pub fn jsonish(key: &str, label: &str) -> Self {
        Self::text(key, label).with_type("jsonish")
    }

    pub fn kv(key: &str, label: &str) -> Self {
        Self::text(key, label).with_type("kv")
    }

    pub fn list(key: &str, label: &str) -> Self {
        Self::text(key, label).with_type("list")
    }

    fn with_type(mut self, field_type: &str) -> Self {
        self.field_type = field_type.into();
        self
    }

    pub fn required(mut self) -> Self {
        self.required = Some(true);
        self
    }

    pub fn placeholder(mut self, value: &str) -> Self {
        self.placeholder = Some(value.into());
        self
    }

    pub fn help(mut self, value: &str) -> Self {
        self.help = Some(value.into());
        self
    }

    pub fn secret(mut self) -> Self {
        self.secret = Some(true);
        self
    }

    pub fn default(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }

    pub fn options(mut self, value: Vec<FieldOption>) -> Self {
        self.options = Some(value);
        self
    }

    pub fn rows(mut self, value: Vec<ListRowField>) -> Self {
        self.rows = Some(value);
        self
    }

    pub fn kv_key_placeholder(mut self, value: &str) -> Self {
        self.kv_key_placeholder = Some(value.into());
        self
    }

    pub fn kv_value_placeholder(mut self, value: &str) -> Self {
        self.kv_value_placeholder = Some(value.into());
        self
    }
}

impl ListRowField {
    pub fn text(key: &str, label: &str) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            field_type: "text".into(),
            placeholder: None,
            options: None,
        }
    }

    pub fn select(key: &str, label: &str) -> Self {
        Self::text(key, label).with_type("select")
    }

    pub fn jsonish(key: &str, label: &str) -> Self {
        Self::text(key, label).with_type("jsonish")
    }

    fn with_type(mut self, field_type: &str) -> Self {
        self.field_type = field_type.into();
        self
    }

    pub fn placeholder(mut self, value: &str) -> Self {
        self.placeholder = Some(value.into());
        self
    }

    pub fn options(mut self, value: Vec<FieldOption>) -> Self {
        self.options = Some(value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Registry;
    use serde_json::json;

    /// Guard against schema/factory drift: every knot's declared schema should
    /// cover the params its factory actually reads. If a new param is added to a
    /// knot, its schema must mention it (otherwise the UI can't edit it).
    #[test]
    fn schemas_cover_factory_params() {
        let registry = Registry::default();

        // kind -> keys the factory reads via params.get(...)
        let expected: &[(&str, &[&str])] = &[
            ("http.request", &["method", "url", "headers", "body"]),
            ("transform.set", &["assignments"]),
            ("transform.delay", &["milliseconds"]),
            ("transform.filter", &["conditions", "mode"]),
            ("notify.log", &["message"]),
            ("notify.resend", &["apiKey", "from", "to", "subject", "html", "text"]),
            ("trigger.webhook", &[]),
            ("trigger.schedule", &[]),
        ];

        for (kind, params) in expected {
            let schema = registry
                .schema(kind)
                .unwrap_or_else(|| panic!("registry missing knot {kind}"));
            let declared: Vec<&str> = schema.iter().map(|f| f.key.as_str()).collect();
            for p in *params {
                assert!(
                    declared.contains(p),
                    "{kind} factory reads param '{p}' but schema does not declare it (schema: {declared:?})"
                );
            }
        }
    }

    #[test]
    fn serializes_camel_case_keys() {
        let field = ParamField::text("url", "URL")
            .required()
            .placeholder("https://x")
            .default(json!("GET"));
        let value = serde_json::to_value(&field).unwrap();
        assert!(value["key"] == json!("url"));
        assert!(value["required"] == json!(true));
        assert!(value["type"] == json!("text"));
    }
}
