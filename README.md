<p align="center">
  <img src="assets/microknot.png" alt="Logo" width="200">
</p>

<h1 align="center">Microknot</h1>

A local-first workflow engine written in Rust. Workflows are defined as JSON, executed by lightweight Rust nodes ("knots"), and orchestrated through a minimal REST API with an embedded Vue dashboard.

## Features

- Local-first: everything runs on your machine, no cloud dependencies
- JSON workflow definitions with validation (cycle detection, connection and registry checks)
- REST API for workflow CRUD and synchronous execution
- Hand-rolled graph editor UI (Vue 3 + TypeScript), embedded into the server binary
- Minimal runtime footprint (SQLite with WAL mode for storage)

## Workflows

A workflow is a set of knots connected by edges. Each knot receives a list of JSON items, processes them, and passes results downstream. A run starts from a seed item (the request body of the run call, or an empty object).

> **Note:** JSON field names are camelCase (`fromOutput` / `toInput`). Using snake_case will silently default port indices to `0`.

```json
{
  "name": "Example",
  "knots": [
    {
      "id": "n1",
      "type": "trigger.webhook",
      "params": {}
    },
    {
      "id": "n2",
      "type": "http.request",
      "params": {
        "method": "GET",
        "url": "https://api.example.com/status"
      }
    },
    {
      "id": "n3",
      "type": "notify.log",
      "params": { "message": "status: {{ $json.status }}" }
    }
  ],
  "connections": [
    { "from": "n1", "fromOutput": 0, "to": "n2", "toInput": 0 },
    { "from": "n2", "fromOutput": 0, "to": "n3", "toInput": 0 }
  ]
}
```

### Knot types

- `trigger.webhook` — entry point; emits the seed item for the run
- `trigger.schedule` — stub; cannot be executed yet (no scheduler implemented)
- `http.request` — call external APIs (templated URL/headers/body)
- `transform.filter` — branch items on conditions (match / no-match outputs)
- `transform.set` — set or overwrite JSON fields via dot-paths
- `transform.delay` — pause execution for a given number of milliseconds
- `notify.log` — print items to the server log
- `notify.resend` — send emails via the Resend API

`ai.call` exists in the code but is not yet registered in the knot registry, so it cannot be used in workflows.

Templating: values support `{{ $json.path.to.field }}` (item data) and `{{ $env.VAR }}` (environment variables).

## Usage

```
microknot serve                    Start the API server and dashboard
microknot serve --addr 127.0.0.1:5050 --db ./microknot.db
microknot validate <file.json>     Validate a workflow definition
microknot list                     List stored workflows
```

Workflows are executed manually via `POST /api/workflows/:id/run`. There is no scheduler, queue, or retry mechanism (yet).

## Development

The frontend is embedded into the binary, so it must be built before the server:

```
cd frontend && pnpm install && pnpm build   # produces frontend/dist
cargo build                                 # embeds frontend/dist via rust-embed
cargo test
cargo run -- serve
```

For frontend development, `vite.config.ts` proxies `/api` to `localhost:5050`, so run `cargo run -- serve` and `pnpm dev` side by side.
