use serde_json::Value;

use crate::item::Item;

pub fn render_param(value: &Value, item: &Item) -> Value {
    match value {
        Value::String(s) => render_string_value(s, item),
        Value::Array(items) => Value::Array(items.iter().map(|v| render_param(v, item)).collect()),
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                out.insert(k.clone(), render_param(v, item));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn render_string_value(s: &str, item: &Item) -> Value {
    let trimmed = s.trim();
    if trimmed.starts_with("{{") && trimmed.ends_with("}}") {
        let expr = &trimmed[2..trimmed.len() - 2];
        resolve(expr, item).unwrap_or(Value::String(String::new()))
    } else {
        Value::String(render_string(s, item))
    }
}

pub fn render_string(s: &str, item: &Item) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < s.len() {
        match s[i..].find("{{") {
            Some(open_rel) => {
                let open = i + open_rel;
                out.push_str(&s[i..open]);
                match s[open + 2..].find("}}") {
                    Some(close_rel) => {
                        let close = open + 2 + close_rel;
                        let expr = &s[open + 2..close];
                        let replacement = resolve(expr, item)
                            .map(|v| match v {
                                Value::String(s) => s,
                                v => v.to_string(),
                            })
                            .unwrap_or_default();
                        out.push_str(&replacement);
                        i = close + 2;
                    }
                    None => {
                        out.push_str(&s[open..]);
                        break;
                    }
                }
            }
            None => {
                out.push_str(&s[i..]);
                break;
            }
        }
    }
    out
}

pub fn resolve(expr: &str, item: &Item) -> Option<Value> {
    let expr = expr.trim();
    let rest = expr.strip_prefix("$json")?;
    let path = rest.trim_start_matches('.');
    if path.is_empty() {
        return Some(item.json.clone());
    }
    resolve_path(&item.json, path.split('.'))
}

pub fn resolve_path<'a>(json: &Value, parts: impl Iterator<Item = &'a str>) -> Option<Value> {
    let mut current = json;
    for part in parts {
        match current {
            Value::Object(map) => current = map.get(part)?,
            Value::Array(arr) => {
                let index: usize = part.parse().ok()?;
                current = arr.get(index)?;
            }
            _ => return None,
        }
    }
    Some(current.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item() -> Item {
        Item::new(json!({ "user": { "name": "ada", "tags": ["x", "y"] }, "count": 3 }))
    }

    #[test]
    fn interpolates_nested_path() {
        let s = "hello {{ $json.user.name }}!";
        assert_eq!(render_string(s, &item()), "hello ada!");
    }

    #[test]
    fn missing_field_renders_empty() {
        assert_eq!(render_string("{{ $json.missing }}", &item()), "");
    }

    #[test]
    fn array_index_access() {
        assert_eq!(render_string("{{ $json.user.tags.1 }}", &item()), "y");
    }

    #[test]
    fn full_interpolation_keeps_type() {
        let v = render_param(&json!("{{ $json.count }}"), &item());
        assert_eq!(v, json!(3));
    }

    #[test]
    fn no_interpolation_passthrough() {
        let v = render_param(&json!("plain"), &item());
        assert_eq!(v, json!("plain"));
    }

    #[test]
    fn renders_nested_objects() {
        let v = render_param(
            &json!({ "path": "{{ $json.user.name }}", "n": [1, "{{ $json.count }}"] }),
            &item(),
        );
        assert_eq!(v, json!({ "path": "ada", "n": [1, 3] }));
    }
}
