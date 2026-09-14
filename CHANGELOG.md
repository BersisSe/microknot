# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] – 2026-09-14

### Added

- Workflow editor rebuilt on `@vue-flow/core`: canvas pan/zoom, knot dragging and
  wire drawing are handled by the library now, with a custom `KnotNode` component
  for rendering knots
- Wire endpoints can be dragged to a different port to rebind a connection, or
  dropped on empty canvas to sever it
- `Delete`/`Backspace` removes the selected knot or connection (ignored while
  typing in form fields)
- Connection validation: self-loops, out-of-range port indices and duplicate
  wires are rejected while drawing
- Run results on the workflow list show a per-knot step table (knot, kind,
  status, item count) instead of a single summary line
- Run accepts an optional seed item as JSON next to the Run button
- `{{ $env.VAR }}` template expression resolves environment variables
- `http.request` caps response bodies at 5 MB instead of reading unbounded
  responses into memory

### Changed

- README now documents what actually exists. The removed claims (queued
  execution, retries with backoff, scheduler, `db.*`/`file.*`/`notify.smtp`
  knots, `microknot run`) were never implemented
- README workflow examples use the camelCase field names the API expects
  (`fromOutput`/`toInput`); the old snake_case examples silently defaulted port
  indices to 0
- Development section documents the required `pnpm build` step before
  `cargo build` (the server embeds `frontend/dist` at compile time)
- CONTRIBUTING: clarified the policy on AI-written code

### Fixed

- Knot layout snapping back to auto-layout positions when a new knot was added
  after dragging nodes around
- Deleted knots and orphaned wires reappearing when another knot was placed
- Newly drawn connections disappearing after the next canvas change
- Clearing a number field in the inspector stored `0` instead of removing the
  parameter
- Error banners that stayed visible after a later action succeeded
- "No workflows yet" flashing while the workflow list was still loading
- Window event listeners in the editor leaking after navigating away

### Removed

- Hand-rolled graph state and wire-geometry code (`useGraphEditor.ts`),
  superseded by the Vue Flow migration

## [0.1.0] – 2026-08-12

### Added

- Workflow engine: JSON-defined workflows of knots connected by typed ports,
  executed in topological order with cycle detection
- Knot types: `trigger.webhook`, `trigger.schedule` (manual-run stub),
  `http.request`, `transform.filter`, `transform.set`, `transform.delay`,
  `ai.call`, `notify.log`, `notify.resend`
- `{{ $json.path }}` templating in knot parameters
- Workflow validation: unique knot ids, dangling and duplicate connections,
  unknown knot types
- SQLite persistence (WAL mode) with schema versioning
- REST API: workflow CRUD, run, knot catalog; duplicate and validation errors
  mapped to 409/422
- CLI: `serve`, `validate`, `list`
- Vue 3 + TypeScript dashboard with a hand-rolled graph editor, embedded into
  the server binary via `rust-embed`
- Unit and integration test suites for the engine, store and HTTP API
- AGPL-3.0 license, CONTRIBUTING guide and GitHub issue templates

[Unreleased]: https://github.com/BersisSe/microknot/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/BersisSe/microknot/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/BersisSe/microknot/releases/tag/v0.1.0
