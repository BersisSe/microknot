use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::item::Item;
use crate::knot::{Ctx, Knot, KnotDescriptor};
use crate::schema::{KnotSchema, ParamField};
use crate::templ;

pub const DESCRIPTOR: KnotDescriptor = KnotDescriptor {
    kind: "notify.resend",
    name: "Resend Email",
    description: "Send an email via the Resend API",
    output_count: 1,
    input_count: 1,
};

pub fn schema() -> KnotSchema {
    vec![
        ParamField::text("apiKey", "API Key")
            .secret()
            .placeholder("re_..."),
        ParamField::text("from", "From")
            .required()
            .placeholder("you@example.com"),
        ParamField::text("to", "To").required().placeholder("them@example.com"),
        ParamField::text("subject", "Subject").required(),
        ParamField::textarea("html", "HTML body"),
        ParamField::textarea("text", "Plain text body"),
    ]
}

const RESEND_URL: &str = "https://api.resend.com/emails";

pub fn factory(params: &Value) -> Box<dyn Knot> {
    let or_default = |key: &str| {
        params
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let optional = |key: &str| {
        params
            .get(key)
            .and_then(Value::as_str)
            .map(String::from)
    };
    Box::new(Resend {
        api_key: or_default("apiKey"),
        from: or_default("from"),
        to: or_default("to"),
        subject: or_default("subject"),
        html: optional("html"),
        text: optional("text"),
        agent: ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .new_agent(),
    })
}

pub struct Resend {
    api_key: String,
    from: String,
    to: String,
    subject: String,
    html: Option<String>,
    text: Option<String>,
    agent: ureq::Agent,
}

impl Knot for Resend {
    fn run(&self, _ctx: &Ctx, input: Vec<Item>) -> Result<Vec<Vec<Item>>> {
        let api_key = if self.api_key.is_empty() {
            std::env::var("RESEND_API_KEY").map_err(|_| {
                Error::Knot("notify.resend requires an `apiKey` param or RESEND_API_KEY env var".into())
            })?
        } else {
            self.api_key.clone()
        };

        let mut out = Vec::with_capacity(input.len());
        for item in input {
            let mut body = json!({
                "from": templ::render_string(&self.from, &item),
                "to": templ::render_string(&self.to, &item),
                "subject": templ::render_string(&self.subject, &item),
            });
            if let Some(html) = &self.html {
                body["html"] = json!(templ::render_string(html, &item));
            }
            if let Some(text) = &self.text {
                body["text"] = json!(templ::render_string(text, &item));
            }

            let request = ureq::http::Request::builder()
                .method("POST")
                .uri(RESEND_URL)
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Content-Type", "application/json")
                .body(body.to_string())
                .map_err(|e| Error::Knot(e.to_string()))?;
            let response = self
                .agent
                .run(request)
                .map_err(|e| Error::Knot(format!("resend request failed: {e}")))?;

            let status = response.status().as_u16();
            let body_text = response
                .into_body()
                .read_to_string()
                .map_err(|e| Error::Knot(e.to_string()))?;
            let info: Value = serde_json::from_str(&body_text)
                .unwrap_or(Value::String(body_text));
            let info = json!({ "status": status, "response": info });

            out.push(Item::new(info));
        }
        Ok(vec![out])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx {
        Ctx { workflow_id: "wf".into() }
    }

    #[test]
    fn missing_key_is_a_fail_fast_error() {
        let knot = factory(&json!({
            "from": "a@example.com",
            "to": "b@example.com",
            "subject": "hi",
        }));
        let result = knot.run(&ctx(), vec![Item::new(json!({}))]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("apiKey"));
    }

    #[test]
    fn no_api_key_and_no_env_var_errors() {
        let knot = factory(&json!({}));
        let result = knot.run(&ctx(), vec![Item::new(json!({}))]);
        assert!(result.is_err());
    }

    #[test]
    fn registered_in_default_registry() {
        let registry = crate::Registry::default();
        let descriptor = registry.descriptor("notify.resend").expect("notify.resend registered");
        assert_eq!(descriptor.output_count, 1);
        assert!(registry.create("notify.resend", &json!({})).is_some());
    }
}