# microknot-vue

Vue 3 + TypeScript + Vite rewrite of the microknot dashboard/editor.

## Why this exists

The original vanilla-JS version had no build step, so a stale or
out-of-sync file (like an `app.js` still calling a deleted `Storage`
object) would fail silently at runtime with no warning. This version
adds:

- **A single source of truth for types** (`src/types.ts`) — the
  `Workflow`/`Knot`/`Connection` shapes are defined once and imported
  everywhere, matching the Rust API exactly (`knots`, not `nodes`;
  `params`, not `config`; connections carry `fromOutput`/`toInput`).
- **A typed API client** (`src/api.ts`) wrapping `fetch` calls to
  `/api/workflows`, `/api/knots`, etc.
- **Compile-time errors instead of runtime silence.** Run `npm run
  build` (which runs `vue-tsc` before `vite build`) and any drift
  between components — wrong prop names, missing imports, wrong types
  — fails the build immediately instead of failing quietly in a
  browser months later.

## Structure

```
src/
  types.ts              Workflow/Knot/Connection types (source of truth)
  api.ts                Typed fetch wrapper around /api/*
  toast.ts              Small reactive toast singleton
  router.ts             Two routes: "/" (list) and "/editor/:id"
  theme.css             Design tokens + all component styling
  useGraphEditor.ts      Composable: node drag/pan/zoom/wires/auto-layout
  views/
    WorkflowList.vue     Workflow list + detail panel (was index.html/app.js)
    WorkflowEditor.vue   Node graph editor (was editor.html/editor.js)
```

## Position / layout

The Rust `Workflow` struct has no x/y field, so canvas position is
**never sent to the server**. `useGraphEditor.ts` auto-lays-out knots
left-to-right by connection depth every time a workflow loads.

## Running

```bash
npm install
npm run dev      # dev server on :5173, proxies /api to localhost:8080
npm run build    # typecheck + production build to dist/
```

Adjust the proxy target in `vite.config.ts` if your Rust server binds
to a different port. In production, serve `dist/` as your static
directory the same way the old `public/` folder was served (feather's
`ServeStatic`) — the API routes are unaffected.
