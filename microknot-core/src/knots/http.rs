use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::item::Item;
use crate::knot::{Ctx, Knot, KnotDescriptor};
use crate::schema::{FieldOption, KnotSchema, ParamField};
use crate::templ;

pub const DESCRIPTOR: KnotDescriptor = KnotDescriptor {
    kind: "http.request",
    name: "HTTP Request",
    description: "Makes an HTTP request per item and emits the response",
    output_count: 1,
    input_count: 1,
};

pub fn schema() -> KnotSchema {
    let methods: Vec<FieldOption> = ["GET", "POST", "PUT", "PATCH", "DELETE"]
        .iter()
        .map(|m| FieldOption { value: m.to_string(), label: m.to_string() })
        .collect();

    vec![
        ParamField::select("method", "Method")
            .options(methods)
            .default(json!("GET")),
        ParamField::text("url", "URL")
            .required()
            .placeholder("https://api.example.com/items"),
        ParamField::kv("headers", "Headers")
            .kv_key_placeholder("header name")
            .kv_value_placeholder("value"),
        ParamField::jsonish("body", "Body (JSON)")
            .help("JSON, or a plain string. Supports {{ $json.field }} templates."),
    ]
}

pub fn factory(params: &Value) -> Box<dyn Knot> {
    let method = params.get("method").and_then(Value::as_str).unwrap_or("GET").to_string();
    let url = params.get("url").and_then(Value::as_str).unwrap_or_default().to_string();
    let headers: Vec<(String, String)> = params
        .get("headers")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
                .collect()
        })
        .unwrap_or_default();
    let body = params.get("body").cloned().unwrap_or(Value::Null);
    let agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .new_agent();
    Box::new(HttpRequest { method, url, headers, body, agent })
}

pub struct HttpRequest {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Value,
    agent: ureq::Agent,
}

impl Knot for HttpRequest {
    fn run(&self, _ctx: &Ctx, input: Vec<Item>) -> Result<Vec<Vec<Item>>> {
        let mut out = Vec::with_capacity(input.len());
        for item in input {
            let url = templ::render_string(&self.url, &item);
            let body = match templ::render_param(&self.body, &item) {
                Value::String(s) => s,
                Value::Null => String::new(),
                v => v.to_string(),
            };
            let mut builder = ureq::http::Request::builder()
                .method(self.method.as_str())
                .uri(&url);
            for (k, v) in &self.headers {
                builder = builder.header(k, templ::render_string(v, &item));
            }
            let request = builder
                .body(body)
                .map_err(|e| Error::Knot(e.to_string()))?;
            let response = self
                .agent
                .run(request)
                .map_err(|e| Error::Knot(e.to_string()))?;

            let status = response.status().as_u16();
            let response_headers: serde_json::Map<String, Value> = response
                .headers()
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_string(),
                        Value::String(value.to_str().unwrap_or_default().to_string()),
                    )
                })
                .collect();
            let body_text = response
                .into_body()
                .read_to_string()
                .map_err(|e| Error::Knot(e.to_string()))?;
            let parsed_body = serde_json::from_str::<Value>(&body_text)
                .unwrap_or(Value::String(body_text));

            out.push(Item::new(Value::Object(serde_json::Map::from_iter([
                ("status".into(), Value::from(status)),
                ("headers".into(), Value::Object(response_headers)),
                ("body".into(), parsed_body),
            ]))));
        }
        Ok(vec![out])
    }
}
