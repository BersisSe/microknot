// Param field types. The schema for each knot now lives server-side (the Rust
// `microknot-core::schema::ParamField`), served in `/api/knots`. These types
// mirror that JSON contract so the inspector can render any knot — including
// new or script-defined ones — without a frontend change.

export interface FieldOption {
  value: string;
  label: string;
}

export interface ListRowField {
  key: string;
  label: string;
  type: "text" | "select" | "jsonish";
  placeholder?: string;
  options?: FieldOption[];
}

export interface ParamField {
  key: string;
  label: string;
  type: "text" | "number" | "select" | "textarea" | "jsonish" | "kv" | "list";
  required?: boolean;
  placeholder?: string;
  help?: string;
  secret?: boolean;
  default?: unknown;
  options?: FieldOption[];
  rows?: ListRowField[];
  kvKeyPlaceholder?: string;
  kvValuePlaceholder?: string;
}

export type KnotSchema = ParamField[];