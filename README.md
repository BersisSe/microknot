<p align="center">
  <img src="assets/microknot.png" alt="Logo" width="200">
</p>

<h1 align="center">Microknot</h1>




A local-first workflow engine written in Rust. Workflows are defined as JSON, executed by lightweight Rust nodes, and orchestrated through a minimal REST API.

## Features

- Local-first: everything runs on your machine, no cloud dependencies
- JSON workflow definitions with validation
- REST API for workflow and execution management
- Synchronous and queued execution modes
- Retries with exponential backoff and timeout handling
- Restart-safe: queued executions resume after a crash
- Minimal runtime footprint

## Workflows

A workflow is a set of nodes connected by edges. Each node receives a list of JSON items, processes them, and passes results downstream.

```json
{
  "name": "Example",
  "knots": [
    {
      "id": "n1",
      "type": "trigger.schedule",
      "params": { "cron": "*/5 * * * *" }
    },
    {
      "id": "n2",
      "type": "http.request",
      "params": {
        "method": "GET",
        "url": "https://api.example.com/status"
      }
    }
  ],
  "connections": [
    { "from": "n1", "from_output": 0, "to": "n2", "to_input": 0 }
  ]
}
```

### Node types

- `trigger.webhook` — start a workflow via an inbound request
- `trigger.schedule` — start a workflow on a cron schedule
- `http.request` — call external APIs
- `transform.filter` — branch on conditions
- `transform.set` — modify JSON fields
- `transform.delay` — pace execution
- `db.sqlite` — run SQL against a database
- `file.read`, `file.write` — read and write local files
- `notify.slack`, `notify.discord`, `notify.smtp`, `notify.log` — notifications

## Usage

```
microknot serve            Start the server, scheduler, and worker
microknot run <file.json>  Execute a workflow headlessly
microknot validate <file.json>
microknot list             List stored workflows
```

## API

| Method | Path | Description |
|---|---|---|
| GET, POST | `/api/workflows` | List or create workflows |
| GET, PUT, DELETE | `/api/workflows/:id` | Manage a workflow |
| POST | `/api/workflows/:id/run` | Execute a workflow synchronously |
| POST | `/api/hooks/:workflow_id/:name` | Webhook trigger entry point |
| GET | `/api/executions` | List executions |
| GET | `/api/executions/:id` | Execution details and per-node telemetry |
| GET | `/api/knots` | Available node types |

## Development

```
cargo build
cargo test
cargo run -- serve
```

## License

AGPLv3
