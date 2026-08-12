use serde_json::{Value, json};

use crate::{Ctx, Item, KnotDescriptor, KnotSchema, ParamField, knot::Knot};

use crate::error::{Error, Result};
use crate::templ;

pub const DESCRIPTOR: KnotDescriptor = KnotDescriptor {
    kind: "ai.call",
    name: "AI Request",
    description: "Call an AI API with templated prompt",
    output_count: 1,
    input_count: 1,
};

pub fn schema() -> KnotSchema {
    vec![
        ParamField::text("apiKey", "API Key").secret(),
        ParamField::text("model", "Model")
            .required()
            .placeholder("gemini-3.5-flash"),
        ParamField::text("endpoint", "Endpoint").required(),
        ParamField::textarea("systemPrompt", "System Prompt").required(),
        ParamField::textarea("userPrompt", "User Prompt").required(),
        ParamField::number("maxTokens", "Max Tokens"),
        ParamField::number("temperature", "Temperature"),
    ]
}

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
            .and_then(Value::as_number)
            .and_then(|n| n.as_i64())
            .map(|n| n as i32)
    };
    
    let optional_float = |key: &str| {
        params
            .get(key)
            .and_then(Value::as_f64)
            .map(|f| f as f32)
    };
    
    Box::new(AI {
        api_key: or_default("apiKey"),
        model: or_default("model"),
        endpoint: or_default("endpoint"),
        system_prompt: or_default("systemPrompt"),
        user_prompt: or_default("userPrompt"),
        max_tokens: optional("maxTokens"),
        temperature: optional_float("temperature"),
        agent: ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .new_agent(),
    })
}

pub struct AI {
    api_key: String,
    model: String,
    endpoint: String,
    system_prompt: String,
    user_prompt: String,
    max_tokens: Option<i32>,
    temperature: Option<f32>,
    agent: ureq::Agent,
}

impl Knot for AI {
    fn run(&self, _ctx: &Ctx, input: Vec<Item>) -> Result<Vec<Vec<Item>>> {
        let mut out = Vec::with_capacity(input.len());

        for item in input {
            let system_prompt = templ::render_string(&self.system_prompt, &item);
            let user_prompt = templ::render_string(&self.user_prompt, &item);
            let mut body = json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt}
                ]
            });
            
            // Add optional fields
            if let Some(tokens) = self.max_tokens {
                body["max_tokens"] = json!(tokens);
            }
            if let Some(temp) = self.temperature {
                body["temperature"] = json!(temp);
            }
            let mut response = self.agent.post(&self.endpoint)
                .header("Authorization", &format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .send(body.to_string())
                .map_err(|e| Error::Knot(format!("AI request failed: {}", e)))?;
            
            // Parse response
            let status = response.status();
            let body_text = response.body_mut().read_to_string()
                .map_err(|e| Error::Knot(format!("Failed to read response: {}", e)))?;
            
            let response_json: Value = serde_json::from_str(&body_text)
                .unwrap_or_else(|_| json!({"error": "Invalid JSON", "raw": body_text}));
            
            // Build output item
            let output = if status.is_success() {
                let content = response_json["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                
                json!({
                    "content": content,
                    "model": self.model,
                    "usage": response_json.get("usage").cloned().unwrap_or(json!(null)),
                    "raw": response_json
                })
            } else {
                json!({
                    "error": format!("HTTP {}", status),
                    "message": body_text,
                    "raw": response_json
                })
            };
            
            out.push(Item::new(output));
        }
        Ok(vec![out])
    }
}
