<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { useRouter, onBeforeRouteLeave } from "vue-router";
import { VueFlow, useVueFlow, type Connection as FlowConnection, type NodeChange } from "@vue-flow/core";
// Local structural types. vue-flow's exported Node/Edge types trigger TS2589
interface FlowEdge {
  id: string;
  source: string;
  target: string;
  sourceHandle?: string | null;
  targetHandle?: string | null;
  style?: any;
  class?: any;
}

interface FlowNode {
  id: string;
  type: string;
  position: { x: number; y: number };
  data: { knot: Knot; descriptor?: KnotDescriptor; onRemove: (id: string) => void };
}

import { Background } from "@vue-flow/background";
import { Controls } from "@vue-flow/controls";
import { api, ApiError } from "../api";
import { showToast } from "../toast";
import { layoutKnots } from "../layout";
import KnotParamsForm from "../KnotParamsForm.vue";
import KnotNode from "../components/KnotNode.vue";
import type { Knot, Connection, KnotDescriptor, Workflow } from "../types";

const props = defineProps<{ id: string }>();
const router = useRouter();

const { screenToFlowCoordinate, removeNodes, findNode } =
  useVueFlow();

const loaded = ref(false);
const errorMessage = ref<string | null>(null);
const workflowName = ref("");
const knotCatalog = ref<KnotDescriptor[]>([]);
let workflowMeta: Workflow | null = null;

const nodes = ref<FlowNode[]>([]);
const edges = ref<FlowEdge[]>([]);
const selectedNodeId = ref<string | null>(null);
const selectedEdgeId = ref<string | null>(null);
const dirty = ref(false);

function markDirty() {
  dirty.value = true;
}

const selectedKnot = computed<Knot | null>(() => {
  if (!selectedNodeId.value) return null;
  return (findNode(selectedNodeId.value)?.data?.knot as Knot) ?? null;
});

const selectedKnotSchema = computed(() => {
  const type = selectedKnot.value?.type;
  if (!type) return [];
  return knotCatalog.value.find((item) => item.kind === type)?.schema ?? [];
});

function applyEdgeSelection() {
  edges.value.forEach((e) => {
    e.class = e.id === selectedEdgeId.value ? "wire selected" : "wire";
  });
}

const selectedConnectionDetail = computed<Connection | null>(() => {
  const id = selectedEdgeId.value;
  if (!id) return null;
  const e = edges.value.find((e) => e.id === id);
  if (!e) return null;
  return {
    from: e.source,
    fromOutput: Number(e.sourceHandle ?? 0),
    to: e.target,
    toInput: Number(e.targetHandle ?? 0),
  };
});

function descriptorOf(nodeId: string): KnotDescriptor | undefined {
  const node = findNode(nodeId);
  const type = (node?.data?.knot as Knot | undefined)?.type;
  return type ? knotCatalog.value.find((d) => d.kind === type) : undefined;
}

async function load() {
  try {
    const [wfBody, knotsBody] = await Promise.all([
      api.getWorkflow(props.id),
      api.listKnots(),
    ]);
    workflowMeta = wfBody.data;
    knotCatalog.value = knotsBody.data;
    workflowName.value = workflowMeta.name;
    const positions = new Map(
      layoutKnots(workflowMeta.knots, workflowMeta.connections).map((p) => [
        p.id,
        p,
      ]),
    );
    nodes.value = workflowMeta.knots.map((knot) => {
      const p = positions.get(knot.id);
      return {
        id: knot.id,
        type: "knot",
        position: { x: p?.x ?? 40, y: p?.y ?? 40 },
        data: { knot, onRemove: removeKnot },
      };
    });
    edges.value = workflowMeta.connections.map((c, i) => ({
      id: `c-${i}`,
      source: c.from,
      sourceHandle: String(c.fromOutput),
      target: c.to,
      targetHandle: String(c.toInput),
      style: { stroke: "#d4a13d", strokeWidth: 1.6, strokeDasharray: "8 4" },
    }));
    loaded.value = true;
  } catch (e) {
    errorMessage.value = e instanceof ApiError ? e.message : String(e);
  }
}

function removeKnot(id: string) {
  nodes.value = nodes.value.filter((n) => n.id !== id);
  edges.value = edges.value.filter((e) => e.source !== id && e.target !== id);
  removeNodes([id]);
  if (selectedNodeId.value === id) selectedNodeId.value = null;
  markDirty();
}

let knotSeq = 1;
function nextKnotId(): string {
  while (nodes.value.some((n) => n.id === `k${knotSeq}`)) knotSeq += 1;
  return `k${knotSeq++}`;
}

function nextEdgeId(): string {
  let n = edges.value.length;
  while (edges.value.some((e) => e.id === `c-${n}`)) n += 1;
  return `c-${n}`;
}

function connectionToEdge(c: FlowConnection): Connection {
  return {
    from: c.source,
    fromOutput: Number(c.sourceHandle ?? 0),
    to: c.target,
    toInput: Number(c.targetHandle ?? 0),
  };
}

function isValidConnection(c: FlowConnection & { id?: string }): boolean {
  if (c.source === c.target) return false;
  const from = descriptorOf(c.source);
  const to = descriptorOf(c.target);
  if (!from || !to) return false;
  if (Number(c.sourceHandle ?? 0) >= from.outputCount) return false;
  if (Number(c.targetHandle ?? 0) >= to.inputCount) return false;
  if (c.id) return true;
  return !edges.value.some(
    (e) =>
      e.source === c.source &&
      Number(e.sourceHandle ?? 0) === Number(c.sourceHandle ?? 0) &&
      e.target === c.target &&
      Number(e.targetHandle ?? 0) === Number(c.targetHandle ?? 0),
  );
}

function onConnect(c: FlowConnection) {
  if (!isValidConnection(c)) return;
  edges.value.push({
    id: nextEdgeId(),
    source: c.source,
    sourceHandle: c.sourceHandle,
    target: c.target,
    targetHandle: c.targetHandle,
    style: { stroke: "#d4a13d", strokeWidth: 1.6, strokeDasharray: "8 4" },
  });
  selectedEdgeId.value = null;
  applyEdgeSelection();
  markDirty();
}

function onNodeClick({ node }: { node: FlowNode }) {
  selectedNodeId.value = node.id;
  selectedEdgeId.value = null;
  applyEdgeSelection();
}

function onEdgeClick({ edge }: { edge: FlowEdge }) {
  selectedEdgeId.value = edge.id;
  selectedNodeId.value = null;
  applyEdgeSelection();
}

function onPaneClick() {
  selectedNodeId.value = null;
  selectedEdgeId.value = null;
  applyEdgeSelection();
}

const edgeUpdatePending = ref<{ id: string } | null>(null);

function onEdgeUpdateStart(e: { edge: FlowEdge }) {
  edgeUpdatePending.value = { id: e.edge.id };
}

function onEdgeUpdate(e: { edge: FlowEdge; connection: FlowConnection }) {
  edgeUpdatePending.value = null;
  const idx = edges.value.findIndex((x) => x.id === e.edge.id);
  if (idx === -1) return;
  const c = e.connection;
  const duplicate = edges.value.some(
    (x) =>
      x.id !== e.edge.id &&
      x.source === c.source &&
      Number(x.sourceHandle ?? 0) === Number(c.sourceHandle ?? 0) &&
      x.target === c.target &&
      Number(x.targetHandle ?? 0) === Number(c.targetHandle ?? 0),
  );
  if (duplicate || !isValidConnection({ ...c })) return;
  edges.value[idx] = {
    id: e.edge.id,
    source: c.source,
    sourceHandle: c.sourceHandle,
    target: c.target,
    targetHandle: c.targetHandle,
    style: e.edge.style,
  };
  markDirty();
}

function onEdgeUpdateEnd(e: { edge: FlowEdge }) {
  if (edgeUpdatePending.value?.id !== e.edge.id) return;
  edgeUpdatePending.value = null;
  edges.value = edges.value.filter((x) => x.id !== e.edge.id);
  if (selectedEdgeId.value === e.edge.id) selectedEdgeId.value = null;
  markDirty();
}

function onKeydown(e: KeyboardEvent) {
  if (e.key !== "Delete" && e.key !== "Backspace") return;
  const t = e.target as HTMLElement | null;
  if (
    t &&
    (t.tagName === "INPUT" ||
      t.tagName === "TEXTAREA" ||
      t.tagName === "SELECT" ||
      t.isContentEditable)
  ) {
    return;
  }
  if (selectedEdgeId.value) {
    e.preventDefault();
    removeSelectedConnection();
  } else if (selectedNodeId.value) {
    e.preventDefault();
    removeKnot(selectedNodeId.value);
  }
}

function onNodesChange(changes: NodeChange[]) {
  changes.forEach((nc) => {
    if (nc.type === "position") {
      const targetNode = nodes.value.find((u) => u.id === nc.id);
      if (targetNode && nc.position) {
        targetNode.position = nc.position;
      }
    }
  })
}

function onCanvasDrop(e: DragEvent) {
  const type = e.dataTransfer?.getData("text/kind");
  if (!type) return;
  const point = screenToFlowCoordinate({ x: e.clientX, y: e.clientY });
  const id = nextKnotId();
  nodes.value.push({
    id,
    type: "knot",
    position: point,
    data: {
      knot: { id, type, params: {} } satisfies Knot,
      descriptor: knotCatalog.value.find((d) => d.kind === type),
      onRemove: removeKnot,
    },
  });
  selectedNodeId.value = id;
  selectedEdgeId.value = null;
  markDirty();
}

function onDragStart(e: DragEvent, kind: string) {
  e.dataTransfer?.setData("text/kind", kind);
}

function removeSelectedConnection() {
  const id = selectedEdgeId.value;
  if (!id) return;
  edges.value = edges.value.filter((e) => e.id !== id);
  selectedEdgeId.value = null;
  markDirty();
}

function removeConnectionTo(c: Connection) {
  const edge = edges.value.find(
    (e) =>
      e.source === c.from &&
      Number(e.sourceHandle ?? 0) === c.fromOutput &&
      e.target === c.to &&
      Number(e.targetHandle ?? 0) === c.toInput,
  );
  if (!edge) return;
  edges.value = edges.value.filter((e) => e.id !== edge.id);
  markDirty();
}

const outgoingConnections = computed(() => {
  const id = selectedKnot.value?.id;
  if (!id) return [];
  return edges.value
    .filter((e) => e.source === id)
    .map((e) => connectionToEdge(e as unknown as FlowConnection));
});

const incomingConnections = computed(() => {
  const id = selectedKnot.value?.id;
  if (!id) return [];
  return edges.value
    .filter((e) => e.target === id)
    .map((e) => connectionToEdge(e as unknown as FlowConnection));
});

async function save() {
  if (!workflowMeta) return;
  const payload = {
    name: workflowName.value || "(untitled)",
    active: workflowMeta.active,
    knots: nodes.value.map((n) => ({ ...(n.data.knot as Knot) })),
    connections: edges.value.map((e) => ({
      from: e.source,
      fromOutput: Number(e.sourceHandle ?? 0),
      to: e.target,
      toInput: Number(e.targetHandle ?? 0),
    })),
  };
  try {
    const body = await api.updateWorkflow(props.id, payload);
    workflowMeta = body.data;
    dirty.value = false;
    showToast("Workflow saved");
  } catch (e) {
    errorMessage.value = e instanceof ApiError ? e.message : String(e);
  }
}

function discard() {
  if (dirty.value && !confirm("Discard changes to this workflow?")) return;
  router.push({ name: "list" });
}

function beforeUnloadHandler(e: BeforeUnloadEvent) {
  if (dirty.value) {
    e.preventDefault();
    e.returnValue = "";
  }
}

onBeforeRouteLeave(() => {
  if (dirty.value) {
    return confirm("Discard changes to this workflow?");
  }
  return true;
});

onMounted(() => {
  load();
  window.addEventListener("beforeunload", beforeUnloadHandler);
  window.addEventListener("keydown", onKeydown);
});

onBeforeUnmount(() => {
  window.removeEventListener("beforeunload", beforeUnloadHandler);
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <header class="topbar">
    <router-link to="/" class="crumb-back">&larr; workflows</router-link>
    <span class="tagline">editor</span>
    <div class="topbar-actions">
      <input
        v-model="workflowName"
        class="muted"
        style="
          font-family: var(--mono);
          font-size: 12px;
          background: var(--bg-sunken);
          border: 1px solid var(--border);
          border-radius: var(--radius);
          padding: 5px 8px;
          color: var(--text);
          width: 200px;
        "
        @input="markDirty"
      />
      <button class="btn" @click="discard">Discard</button>
      <button class="btn btn-primary" :disabled="!loaded" @click="save">
        Save workflow
      </button>
    </div>
  </header>

  <div class="banner" :class="{ hidden: !errorMessage }">
    Error: {{ errorMessage }}
  </div>

  <div v-if="loaded" class="editor-shell">
    <aside class="palette">
      <div class="palette-header">Knot catalog</div>
      <div>
        <div
          v-for="item in knotCatalog"
          :key="item.kind"
          class="palette-item"
          draggable="true"
          @dragstart="onDragStart($event, item.kind)"
        >
          <span class="p-kind">{{ item.kind }}</span>
          <span class="p-desc">{{ item.description || item.name }}</span>
        </div>
      </div>
      <div class="palette-hint">
        drag a knot onto&#10;the canvas to add it.&#10;drag from a port
        to&#10;connect two knots.
      </div>
    </aside>

    <div class="canvas-wrap" @drop="onCanvasDrop" @dragover.prevent>
      <div class="canvas">
        <VueFlow
          :nodes="nodes as any"
          :edges="edges as any"
          :delete-key-code="null"
          :edges-updatable="true"
          :connection-radius="24"
          fit-view-on-init
          @connect="onConnect"
          :is-valid-connection="isValidConnection"
          @node-click="onNodeClick"
          @edge-click="onEdgeClick"
          @pane-click="onPaneClick"
          @node-drag-stop="markDirty"
          @edge-change="markDirty"
          @nodes-change="onNodesChange"
          @edge-update-start="onEdgeUpdateStart"
          @edge-update="onEdgeUpdate"
          @edge-update-end="onEdgeUpdateEnd"
        >
          <template #node-knot="nodeProps">
            <KnotNode v-bind="nodeProps" />
          </template>
          <Background :gap="22" pattern-color="var(--border)" />
          <Controls position="top-left" />
        </VueFlow>
        <div v-if="nodes.length === 0" class="empty-canvas">
          drag a knot from the catalog to begin
        </div>
      </div>
    </div>

    <aside class="inspector">
      <div class="inspector-header">Inspector</div>
      <div
        v-if="!selectedKnot && !selectedConnectionDetail"
        class="empty"
        style="padding: 12px 0; text-align: left"
      >
        Select a knot to edit its params, or click a wire to cut it.
      </div>
      <template v-if="selectedConnectionDetail">
        <div class="field">
          <label>Selected connection</label>
          <div class="conn-row">
            <span
              >{{ selectedConnectionDetail.from }} →
              {{ selectedConnectionDetail.to }}</span
            >
            <button
              class="btn btn-small btn-danger"
              @click="removeSelectedConnection"
            >
              Cut
            </button>
          </div>
          <div class="muted" style="font-size: 10px; margin-top: 4px">
            out {{ selectedConnectionDetail.fromOutput }} → in
            {{ selectedConnectionDetail.toInput }}
          </div>
        </div>
      </template>
      <template v-else-if="selectedKnot">
        <div class="field">
          <label>Type</label><input :value="selectedKnot.type" disabled />
        </div>
        <div class="field">
          <label>ID</label><input :value="selectedKnot.id" disabled />
        </div>
        <KnotParamsForm
          :knot="selectedKnot"
          :schema="selectedKnotSchema"
          :key="selectedKnot.id"
          style="margin-top: 2px"
          @change="markDirty"
        />
        <div class="field">
          <label>Connections out ({{ outgoingConnections.length }})</label>
          <div
            v-for="(c, idx) in outgoingConnections"
            :key="idx"
            class="conn-row"
          >
            <span
              >&rarr; {{ c.to }} (out {{ c.fromOutput }} &rarr; in
              {{ c.toInput }})</span
            >
            <button @click="removeConnectionTo(c)">&times;</button>
          </div>
          <div v-if="outgoingConnections.length === 0" class="muted">none</div>
        </div>
        <div class="field">
          <label>Connections in ({{ incomingConnections.length }})</label>
          <div
            v-for="(c, idx) in incomingConnections"
            :key="idx"
            class="conn-row"
          >
            <span
              >&larr; {{ c.from }} (out {{ c.fromOutput }} &rarr; in
              {{ c.toInput }})</span
            >
          </div>
          <div v-if="incomingConnections.length === 0" class="muted">none</div>
        </div>
      </template>
    </aside>
  </div>

  <div v-else-if="!errorMessage" class="empty">Loading…</div>
</template>
