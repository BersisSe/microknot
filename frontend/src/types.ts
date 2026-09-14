// Shared types — mirrors the Rust `Workflow` struct and /api/knots response
// exactly. This is the single source of truth: every component imports
// from here instead of redefining the shape locally, which is what caused
// the drift (nodes vs knots, updatedAt that doesn't exist, etc.) in the
// old vanilla-JS version.

export interface Connection {
  from: string;
  fromOutput: number;
  to: string;
  toInput: number;
}

import type { ParamField } from "./knotSchema";

export interface Knot {
  id: string;
  name?: string;
  type: string;
  params: Record<string, unknown>;
  position?: [number, number];
}

export interface Workflow {
  id: string;
  name: string;
  active: boolean;
  knots: Knot[];
  connections: Connection[];
}

// What POST /api/workflows accepts — no id (server assigns it).
export type WorkflowInput = Omit<Workflow, "id">;

export interface KnotDescriptor {
  kind: string;
  name: string;
  description: string;
  inputCount: number;
  outputCount: number;
  schema: ParamField[];
}

export interface RunStep {
  knot: string;
  kind: string;
  status: string;
  items: number;
}

export interface RunResult {
  status: string;
  steps: RunStep[];
}

// Client-only: a Knot plus canvas position, used only inside the editor.
// Never sent to the server — position has no field in the Rust struct.
export interface LaidOutKnot extends Knot {
  x: number;
  y: number;
}
