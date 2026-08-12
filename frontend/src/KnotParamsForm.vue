<script setup lang="ts">
import { ref } from "vue";
import type { Knot } from "./types";
import type { ParamField, ListRowField } from "./knotSchema";

const props = defineProps<{ knot: Knot; schema: ParamField[] }>();
const emit = defineEmits<{ change: [] }>();

// schema comes from the server (KnotDescriptor.schema) and drives the form.
const schema = props.schema;

// params is a reactive object owned by the graph's knots array, so mutating it
// directly is reactive. We only signal the parent to mark the workflow dirty.
const params = props.knot.params as Record<string, unknown>;

function commit() {
  emit("change");
}

// ---- scalar helpers ----

function setScalar(key: string, value: unknown) {
  if (typeof value === "string" && value === "") delete params[key];
  else params[key] = value;
  commit();
}

function onText(key: string, e: Event) {
  setScalar(key, (e.target as HTMLInputElement).value);
}

function onNumber(key: string, e: Event) {
  const n = Number((e.target as HTMLInputElement).value);
  params[key] = Number.isNaN(n) ? 0 : n;
  commit();
}

function onSelect(key: string, e: Event) {
  setScalar(key, (e.target as HTMLSelectElement).value);
}

// ---- jsonish fields (parse into a JSON value, fall back to raw string) ----

// Keep the raw editor text in a map so typing doesn't reflow the caret while we
// re-derive the stored value. Keyed by `${key}:${rowIdx}:${rowField}` for rows.
const rawText = ref<Record<string, string>>({});

function rawKey(field: string, rowIdx?: number, rowField?: string) {
  return rowIdx === undefined ? field : `${field}:${rowIdx}:${rowField}`;
}

function parseJsonish(text: string): unknown {
  const t = text.trim();
  if (t === "") return undefined;
  try {
    return JSON.parse(t);
  } catch {
    return text;
  }
}

function displayJsonish(value: unknown): string {
  if (value === undefined || value === null) return "";
  if (typeof value === "string") return value;
  return JSON.stringify(value, null, 2);
}

function onJsonishInput(key: string, e: Event) {
  const el = e.target as HTMLTextAreaElement;
  rawText.value[key] = el.value;
  const parsed = parseJsonish(el.value);
  if (parsed === undefined) delete params[key];
  else params[key] = parsed;
  commit();
}

// ---- kv fields (e.g. http headers) ----

const kvRows = ref<Record<string, { k: string; v: string }[]>>({});

function kvOf(key: string) {
  if (!kvRows.value[key]) {
    const obj = params[key];
    const entries =
      typeof obj === "object" && obj !== null
        ? Object.entries(obj as Record<string, unknown>)
        : [];
    kvRows.value[key] = entries.map(([k, v]) => ({
      k,
      v: typeof v === "string" ? v : JSON.stringify(v),
    }));
  }
  return kvRows.value[key];
}

function syncKv(key: string) {
  const obj: Record<string, string> = {};
  for (const row of kvRows.value[key]) {
    const k = row.k.trim();
    if (k) obj[k] = row.v;
  }
  if (Object.keys(obj).length === 0) delete params[key];
  else params[key] = obj;
  commit();
}

function addKvRow(key: string) {
  kvOf(key).push({ k: "", v: "" });
}

function removeKvRow(key: string, idx: number) {
  kvOf(key).splice(idx, 1);
  syncKv(key);
}

// ---- list fields (e.g. assignments, conditions) ----

function listOf(key: string): Record<string, unknown>[] {
  const v = params[key];
  return Array.isArray(v) ? (v as Record<string, unknown>[]) : [];
}

function addRow(key: string, field: ParamField) {
  const rows = listOf(key);
  const row: Record<string, unknown> = {};
  for (const rf of field.rows ?? []) {
    row[rf.key] = rf.type === "select" && rf.options ? rf.options[0].value : "";
  }
  rows.push(row);
  params[key] = rows;
  commit();
}

function removeRow(key: string, idx: number) {
  const rows = listOf(key);
  rows.splice(idx, 1);
  if (rows.length === 0) delete params[key];
  else params[key] = rows;
  commit();
}

function onRowInput(key: string, idx: number, rf: ListRowField, e: Event) {
  const el = e.target as HTMLInputElement;
  const rows = listOf(key);
  const row = rows[idx];
  if (rf.type === "jsonish") {
    const rk = rawKey(key, idx, rf.key);
    rawText.value[rk] = el.value;
    const parsed = parseJsonish(el.value);
    if (parsed === undefined) delete row[rf.key];
    else row[rf.key] = parsed;
  } else {
    row[rf.key] = el.value;
  }
  params[key] = rows;
  commit();
}

function displayRowValue(key: string, idx: number, rf: ListRowField): string {
  const rk = rawKey(key, idx, rf.key);
  if (rawText.value[rk] !== undefined) return rawText.value[rk];
  const rows = listOf(key);
  return displayJsonish(rows[idx]?.[rf.key]);
}

function displayTopLevel(key: string): string {
  if (rawText.value[key] !== undefined) return rawText.value[key];
  return displayJsonish(params[key]);
}
</script>

<template>
  <div v-if="schema.length === 0" class="muted" style="padding: 8px 0;">
    This knot takes no parameters.
  </div>

  <template v-else>
    <div v-for="field in schema" :key="field.key" class="field" :class="`pf-${field.type}`">
      <label>
        {{ field.label }}
        <span v-if="field.required" class="pf-required">*</span>
      </label>

      <!-- text -->
      <input
        v-if="field.type === 'text'"
        :type="field.secret ? 'password' : 'text'"
        :value="(params[field.key] as string) ?? ''"
        :placeholder="field.placeholder"
        autocomplete="off"
        spellcheck="false"
        @input="onText(field.key, $event)"
      />

      <!-- number -->
      <input
        v-else-if="field.type === 'number'"
        type="number"
        :value="(params[field.key] as number) ?? field.default ?? 0"
        step="1"
        @input="onNumber(field.key, $event)"
      />

      <!-- select -->
      <select
        v-else-if="field.type === 'select'"
        :value="(params[field.key] as string) ?? field.default ?? field.options?.[0]?.value ?? ''"
        @change="onSelect(field.key, $event)"
      >
        <option v-for="opt in field.options" :key="opt.value" :value="opt.value">
          {{ opt.label }}
        </option>
      </select>

      <!-- textarea -->
      <textarea
        v-else-if="field.type === 'textarea'"
        :value="(params[field.key] as string) ?? ''"
        :placeholder="field.placeholder"
        spellcheck="false"
        rows="3"
        @input="setScalar(field.key, ($event.target as HTMLTextAreaElement).value)"
      ></textarea>

      <!-- jsonish -->
      <textarea
        v-else-if="field.type === 'jsonish'"
        :value="displayTopLevel(field.key)"
        :placeholder="field.placeholder"
        spellcheck="false"
        rows="4"
        @input="onJsonishInput(field.key, $event)"
      ></textarea>

      <!-- kv -->
      <div v-else-if="field.type === 'kv'" class="kv-list">
        <div v-for="(row, idx) in kvOf(field.key)" :key="idx" class="kv-row">
          <input
            type="text"
            v-model="row.k"
            :placeholder="field.kvKeyPlaceholder"
            spellcheck="false"
            @input="syncKv(field.key)"
          />
          <input
            type="text"
            v-model="row.v"
            :placeholder="field.kvValuePlaceholder"
            spellcheck="false"
            @input="syncKv(field.key)"
          />
          <button class="row-remove" title="remove" @click="removeKvRow(field.key, idx)">&times;</button>
        </div>
        <button class="btn btn-small" @click="addKvRow(field.key)">+ Add row</button>
      </div>

      <!-- list -->
      <div v-else-if="field.type === 'list'" class="list">
        <div v-for="(row, idx) in listOf(field.key)" :key="idx" class="list-row">
          <div v-for="rf in field.rows ?? []" :key="rf.key" class="lr-field">
            <span class="lr-label">{{ rf.label }}</span>
            <select
              v-if="rf.type === 'select'"
              :value="(row[rf.key] as string) ?? rf.options?.[0]?.value ?? ''"
              @change="onRowInput(field.key, idx, rf, $event)"
            >
              <option v-for="opt in rf.options" :key="opt.value" :value="opt.value">
                {{ opt.label }}
              </option>
            </select>
            <input
              v-else
              type="text"
              :value="rf.type === 'jsonish' ? displayRowValue(field.key, idx, rf) : (row[rf.key] as string) ?? ''"
              :placeholder="rf.placeholder"
              spellcheck="false"
              @input="onRowInput(field.key, idx, rf, $event)"
            />
          </div>
          <button class="row-remove" title="remove" @click="removeRow(field.key, idx)">&times;</button>
        </div>
        <button class="btn btn-small" @click="addRow(field.key, field)">+ Add row</button>
      </div>

      <div v-if="field.help" class="pf-help">{{ field.help }}</div>
    </div>
  </template>
</template>

<style scoped>
.pf-required {
  color: var(--danger);
}
.pf-help {
  margin-top: 4px;
  font-size: 10px;
  line-height: 1.5;
  color: var(--text-dim);
}

.kv-list,
.list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.kv-row,
.list-row {
  display: flex;
  align-items: center;
  gap: 5px;
}
.kv-row input {
  flex: 1;
}
.row-remove {
  flex: 0 0 auto;
  background: none;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  font-size: 13px;
  line-height: 1;
  padding: 2px 4px;
}
.row-remove:hover {
  color: var(--danger);
}

.list-row {
  align-items: flex-start;
  padding: 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-sunken);
}
.lr-field {
  flex: 1;
  min-width: 0;
}
.lr-label {
  display: block;
  font-family: var(--mono);
  font-size: 9px;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--text-dim);
  margin-bottom: 3px;
}
.list-row .lr-field + .lr-field {
  margin-left: 6px;
}
</style>
