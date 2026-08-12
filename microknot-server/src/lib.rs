use feather::{Finalizer, Response, Router, State, json, middleware, next, end};
use microknot_core::{Error as CoreError, Store, ValidationError, Workflow};
use serde_json::Value;

pub fn api_router() -> Router {
    let mut router = Router::new();

    router.get("/health", middleware!(|_req, res, _ctx| {
        res.set_status(200).finish_json(&json!({"status": "OK"}))
    }));

    router.get("/workflows", middleware!(|_req, res, ctx| {
        let store = ctx.get_state::<State<Store>>();
        let workflows = store.with_scope(|store| store.list_workflows());
        match workflows {
            Ok(workflows) => send_json(res, 200, &json!({"data": workflows})),
            Err(e) => internal_error(res, &e),
        }
    }));

    router.post("/workflows", middleware!(|req, res, ctx| {
        let store = ctx.get_state::<State<Store>>();
        let workflow: Workflow = match serde_json::from_slice(&req.body) {
            Ok(workflow) => workflow,
            Err(e) => {
                return send_json(
                    res,
                    400,
                    &json!({"error": format!("invalid JSON body: {e}")}),
                )
            }
        };
        if let Err(errors) = workflow.validate() {
            return validation_error(res, errors);
        }

        let result = store.with_scope(|store| store.insert_workflow(&workflow));
        match result {
            Ok(()) => send_json(res, 201, &json!({"data": workflow})),
            Err(CoreError::Duplicate { id }) => {
                send_json(res, 409, &json!({"error": format!("workflow '{id}' already exists")}))
            }
            Err(e) => internal_error(res, &e),
        }
    }));

    router.get("/workflows/:id", middleware!(|req, res, ctx| {
        let store = ctx.get_state::<State<Store>>();
        let id = match req.param("id") {
            Some(id) => id,
            None => return send_json(res, 400, &json!({"error": "missing workflow id"})),
        };
        match store.with_scope(|store| store.get_workflow(id)) {
            Ok(Some(workflow)) => send_json(res, 200, &json!({"data": workflow})),
            Ok(None) => not_found(res, &format!("workflow '{id}' not found")),
            Err(e) => internal_error(res, &e),
        }
    }));

    router.put("/workflows/:id", middleware!(|req, res, ctx| {
        let store = ctx.get_state::<State<Store>>();
        let id = match req.param("id") {
            Some(id) => id,
            None => return send_json(res, 400, &json!({"error": "missing workflow id"})),
        };
        let mut workflow: Workflow = match serde_json::from_slice(&req.body) {
            Ok(workflow) => workflow,
            Err(e) => {
                return send_json(
                    res,
                    400,
                    &json!({"error": format!("invalid JSON body: {e}")}),
                )
            }
        };
        workflow.id = id.to_string();
        if let Err(errors) = workflow.validate() {
            return validation_error(res, errors);
        }
        let result = store.with_scope(|store| store.update_workflow(&workflow));
        match result {
            Ok(()) => send_json(res, 200, &json!({"data": workflow})),
            Err(CoreError::NotFound { id }) => {
                not_found(res, &format!("workflow '{id}' not found"))
            }
            Err(e) => internal_error(res, &e),
        }
    }));

    router.delete("/workflows/:id", middleware!(|req, res, ctx| {
        let store = ctx.get_state::<State<Store>>();
        let id = match req.param("id") {
            Some(id) => id,
            None => return send_json(res, 400, &json!({"error": "missing workflow id"})),
        };
        match store.with_scope(|store| store.delete_workflow(id)) {
            Ok(true) => {
                res.set_status(204);
                res.finish_bytes(Vec::new())
            }
            Ok(false) => not_found(res, &format!("workflow '{id}' not found")),
            Err(e) => internal_error(res, &e),
        }
    }));

    router.get("/knots", middleware!(|_req, res, _ctx| {
        let registry = microknot_core::Registry::default();
        let knots: Vec<Value> = registry
            .catalog()
            .into_iter()
            .map(|(d, schema)| {
                json!({
                    "kind": d.kind,
                    "name": d.name,
                    "description": d.description,
                    "inputCount": d.input_count,
                    "outputCount": d.output_count,
                    "schema": schema,
                })
            })
            .collect();
        send_json(res, 200, &json!({"data": knots}))
    }));

    router.post("/workflows/:id/run", middleware!(|req, res, ctx| {
        let store = ctx.get_state::<State<Store>>();
        let id = match req.param("id") {
            Some(id) => id,
            None => return send_json(res, 400, &json!({"error": "missing workflow id"})),
        };
        let workflow = match store.with_scope(|store| store.get_workflow(id)) {
            Ok(Some(workflow)) => workflow,
            Ok(None) => return not_found(res, &format!("workflow '{id}' not found")),
            Err(e) => return internal_error(res, &e),
        };
        let body: Option<Value> = if req.body.is_empty() {
            None
        } else {
            match serde_json::from_slice(&req.body) {
                Ok(value) => Some(value),
                Err(e) => {
                    return send_json(
                        res,
                        400,
                        &json!({"error": format!("invalid JSON body: {e}")}),
                    )
                }
            }
        };
        let registry = microknot_core::Registry::default();
        match microknot_core::run_workflow(&workflow, &registry, body) {
            Ok(result) => send_json(res, 200, &json!({"data": result})),
            Err(e) => internal_error(res, &e),
        }
    }));

    router
}

fn send_json(res: &mut Response, status: u16, value: &Value) -> feather::Outcome {
    res.set_status(status);
    res.finish_json(value)
}

fn validation_error(res: &mut Response, errors: Vec<ValidationError>) -> feather::Outcome {
    let details: Vec<String> = errors.into_iter().map(|e| e.message).collect();
    send_json(res, 422, &json!({"error": "invalid workflow", "details": details}))
}

fn not_found(res: &mut Response, message: &str) -> feather::Outcome {
    send_json(res, 404, &json!({"error": message}))
}

fn internal_error(res: &mut Response, e: &CoreError) -> feather::Outcome {
    send_json(res, 500, &json!({"error": format!("internal error: {e}")}))
}

pub fn serve(addr: &str, store: Store) {
    let mut app = feather::App::new();
    app.context().set_state(feather::State::new(store));
    app.set_error_handler(Box::new(|err, _req, res| {
        res.set_status(500);
        let _ = res.finish_json(&json!({"error": format!("{err}")}));
    }));

    // Serve the embedded frontend. Non-API requests resolve against the built
    // `frontend/dist` assets baked into the binary; unknown non-asset paths fall
    // back to index.html so client-side routes survive a hard refresh.
    app.use_middleware(middleware!(|req, res, _ctx| {
        let path = req.uri.path();
        if path.starts_with("/api") {
            return next!();
        }
        let relative = path.trim_start_matches('/');
        let bytes: Option<Vec<u8>> = Embedded::get(relative).map(|f| f.data.into_owned());
        match bytes {
            Some(bytes) => {
                let content_type = guess_content_type(relative);
                res.add_header("Content-Type", content_type).ok();
                res.set_status(200);
                res.send_bytes(bytes);
                end!()
            }
            None => {
                // SPA fallback: serve the dashboard for any unknown non-asset path.
                match Embedded::get("index.html") {
                    Some(index) => {
                        res.add_header("Content-Type", "text/html; charset=utf-8").ok();
                        res.set_status(200);
                        res.send_bytes(index.data.into_owned());
                        end!()
                    }
                    None => next!(),
                }
            }
        }
    }));

    app.mount("/api", api_router());
    app.listen(addr);
}

#[derive(rust_embed::RustEmbed)]
#[folder = "../frontend/dist"]
struct Embedded;

fn guess_content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

