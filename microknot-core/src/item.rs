use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub json: Value,
}

impl Item {
    pub fn new(json: Value) -> Self {
        Self { json }
    }
}

impl From<Value> for Item {
    fn from(json: Value) -> Self {
        Self { json }
    }
}
