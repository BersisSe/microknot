use std::sync::Once;
use std::thread;
use std::time::Duration;

use microknot_core::Store;
use serde_json::Value;
use ureq::http;

const BASE: &str = "http://127.0.0.1:5599/api";

static START: Once = Once::new();

fn start_server() {
    START.call_once(|| {
        let store = Store::open_in_memory().unwrap();
        let addr = "127.0.0.1:5599".to_string();
        thread::spawn(move || microknot_server::serve(&addr, store));
        for _ in 0..40 {
            if ureq::get(&format!("{BASE}/health")).call().is_ok() {
                return;
            }
            thread::sleep(Duration::from_millis(250));
        }
        panic!("server did not become ready");
    });
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .new_agent()
}

fn read(resp: http::Response<ureq::Body>) -> (u16, Value) {
    let status = resp.status().as_u16();
    let text = resp.into_body().read_to_string().unwrap_or_default();
    let value = if text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).expect("response was not JSON")
    };
    (status, value)
}

fn post_workflow(agent: &ureq::Agent, body: &str) -> (u16, Value) {
    let resp = agent
        .post(&format!("{BASE}/workflows"))
        .header("Content-Type", "application/json")
        .send(body)
        .expect("transport error");
    read(resp)
}

fn get_workflow(agent: &ureq::Agent, id: &str) -> (u16, Value) {
    read(
        agent
            .get(&format!("{BASE}/workflows/{id}"))
            .call()
            .expect("transport error"),
    )
}

fn put_workflow(agent: &ureq::Agent, id: &str, body: &str) -> (u16, Value) {
    let resp = agent
        .put(&format!("{BASE}/workflows/{id}"))
        .header("Content-Type", "application/json")
        .send(body)
        .expect("transport error");
    read(resp)
}

fn delete_workflow(agent: &ureq::Agent, id: &str) -> (u16, Value) {
    read(
        agent
            .delete(&format!("{BASE}/workflows/{id}"))
            .call()
            .expect("transport error"),
    )
}

fn valid_body(id: &str) -> String {
    format!(
        r#"{{"id":"{id}","name":"Test {id}","knots":[{{"id":"a","type":"trigger.webhook"}},{{"id":"b","type":"notify.log"}}],"connections":[{{"from":"a","to":"b"}}]}}"#
    )
}

#[test]
fn health_ok() {
    start_server();
    let resp = ureq::get(&format!("{BASE}/health")).call().unwrap();
    assert_eq!(resp.status().as_u16(), 200);
}

#[test]
fn create_assigns_generated_id() {
    start_server();
    let agent = agent();
    let (status, body) = post_workflow(
        &agent,
        r#"{"name":"Generated","knots":[{"id":"a","type":"trigger.webhook"}]}"#,
    );
    assert_eq!(status, 201);
    let id = body["data"]["id"].as_str().unwrap().to_string();
    assert!(!id.is_empty());
    assert_eq!(get_workflow(&agent, &id).0, 200);
}

#[test]
fn create_then_get_round_trip() {
    start_server();
    let agent = agent();
    let (status, created) = post_workflow(&agent, &valid_body("wf_roundtrip"));
    assert_eq!(status, 201);
    assert_eq!(created["data"]["name"], "Test wf_roundtrip");

    let (status, got) = get_workflow(&agent, "wf_roundtrip");
    assert_eq!(status, 200);
    assert_eq!(got["data"]["knots"].as_array().unwrap().len(), 2);
    assert_eq!(got["data"]["connections"].as_array().unwrap().len(), 1);
}

#[test]
fn create_duplicate_id_conflicts() {
    start_server();
    let agent = agent();
    let body = valid_body("wf_dup");
    assert_eq!(post_workflow(&agent, &body).0, 201);
    let (status, err) = post_workflow(&agent, &body);
    assert_eq!(status, 409);
    assert!(err["error"].as_str().unwrap().contains("already exists"));
}

#[test]
fn put_replaces_workflow() {
    start_server();
    let agent = agent();
    let body = valid_body("wf_put");
    assert_eq!(post_workflow(&agent, &body).0, 201);
    let updated = r#"{"name":"Renamed","knots":[{"id":"a","type":"notify.log"}]}"#;
    let (status, body) = put_workflow(&agent, "wf_put", updated);
    assert_eq!(status, 200);
    assert_eq!(body["data"]["name"], "Renamed");
}

#[test]
fn delete_then_404() {
    start_server();
    let agent = agent();
    let body = valid_body("wf_delete");
    assert_eq!(post_workflow(&agent, &body).0, 201);
    assert_eq!(delete_workflow(&agent, "wf_delete").0, 204);
    assert_eq!(get_workflow(&agent, "wf_delete").0, 404);
}

#[test]
fn get_missing_is_404() {
    start_server();
    let agent = agent();
    let (status, body) = get_workflow(&agent, "no_such_workflow");
    assert_eq!(status, 404);
    assert!(body["error"].is_string());
}

#[test]
fn bad_json_is_400() {
    start_server();
    let agent = agent();
    let (status, _) = post_workflow(&agent, "this is not json");
    assert_eq!(status, 400);
}

#[test]
fn invalid_workflow_is_422() {
    start_server();
    let agent = agent();
    let (status, body) = post_workflow(&agent, r#"{"name":"Empty","knots":[]}"#);
    assert_eq!(status, 422);
    assert!(body["details"].as_array().is_some());
}

#[test]
fn knots_catalog_lists_types() {
    start_server();
    let agent = agent();
    let (status, body) = read(
        agent
            .get(&format!("{BASE}/knots"))
            .call()
            .expect("transport error"),
    );
    assert_eq!(status, 200);
    let knots = body["data"].as_array().unwrap();
    assert!(knots.len() >= 7);
    let kinds: Vec<String> = knots
        .iter()
        .map(|k| k["kind"].as_str().unwrap().to_string())
        .collect();
    assert!(kinds.contains(&"http.request".to_string()));
    assert!(kinds.contains(&"transform.filter".to_string()));

    // Every knot must expose a `schema` array. Parameterized knots describe
    // their fields so the UI form renders without frontend knowledge; triggers
    // that take no params legitimately return an empty list.
    let mut with_fields = 0;
    for k in knots {
        let schema = k["schema"].as_array().unwrap();
        if !schema.is_empty() {
            with_fields += 1;
        }
    }
    assert!(
        with_fields >= 6,
        "expected most knots to declare params, got {with_fields}"
    );

    let http = knots.iter().find(|k| k["kind"] == "http.request").unwrap();
    let keys: Vec<String> = http["schema"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["key"].as_str().unwrap().to_string())
        .collect();
    assert!(keys.contains(&"url".to_string()));
    assert!(keys.contains(&"method".to_string()));
}

#[test]
fn run_workflow_endpoint() {
    start_server();
    let agent = agent();
    let (status, body) = post_workflow(
        &agent,
        r#"{"name":"Run","knots":[{"id":"a","type":"trigger.webhook"},{"id":"b","type":"transform.set","params":{"assignments":[{"field":"echo","value":"{{ $json.input }}"}]}},{"id":"c","type":"notify.log"}],"connections":[{"from":"a","to":"b"},{"from":"b","to":"c"}]}"#,
    );
    assert_eq!(status, 201);
    let id = body["data"]["id"].as_str().unwrap();

    let (status, body) = read(
        agent
            .post(&format!("{BASE}/workflows/{id}/run"))
            .header("Content-Type", "application/json")
            .send(r#"{"input":"hi"}"#)
            .expect("transport error"),
    );
    assert_eq!(status, 200);
    assert_eq!(body["data"]["status"], "ok");
    assert_eq!(body["data"]["steps"].as_array().unwrap().len(), 3);
}

#[test]
fn run_missing_workflow_returns_404() {
    start_server();
    let agent = agent();
    let (status, body) = read(
        agent
            .post(&format!("{BASE}/workflows/nope/run"))
            .send("")
            .expect("transport error"),
    );
    assert_eq!(status, 404);
    assert!(body["error"].is_string());
}
