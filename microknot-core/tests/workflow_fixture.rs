use microknot_core::{Store, Workflow};

const FIXTURE: &str = include_str!("fixtures/example.json");

#[test]
fn fixture_parses_and_validates() {
    let wf: Workflow = serde_json::from_str(FIXTURE).unwrap();
    wf.validate().unwrap();
    assert_eq!(wf.knots.len(), 3);
    assert_eq!(wf.connections.len(), 2);
}

#[test]
fn fixture_round_trips_through_store() {
    let store = Store::open_in_memory().unwrap();
    let wf: Workflow = serde_json::from_str(FIXTURE).unwrap();
    store.insert_workflow(&wf).unwrap();

    let loaded = store.get_workflow(&wf.id).unwrap().unwrap();
    assert_eq!(
        serde_json::to_string(&loaded).unwrap(),
        serde_json::to_string(&wf).unwrap()
    );
}
