# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
  knots, `microknot run`) were never implemented but hopefully would be soon...
- README workflow examples use the camelCase field names the API expects
  (`fromOutput`/`toInput`); the old snake_case examples silently defaulted port
  indices to 0
- Development section documents the required `pnpm build` step before
  `cargo build` (the server embeds `frontend/dist` at compile time)

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
