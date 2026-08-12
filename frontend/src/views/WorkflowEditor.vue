<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { useRouter, onBeforeRouteLeave } from "vue-router";
import { api, ApiError } from "../api";
import { showToast } from "../toast";
import { useGraphEditor } from "../useGraphEditor";
import KnotParamsForm from "../KnotParamsForm.vue";
import type { KnotDescriptor, Workflow } from "../types";

const props = defineProps<{ id: string }>();
const router = useRouter();

const loaded = ref(false);
const errorMessage = ref<string | null>(null);
const workflowName = ref("");
const knotCatalog = ref<KnotDescriptor[]>([]);
let workflowMeta: Workflow | null = null;

let graph: ReturnType<typeof useGraphEditor> | null = null;
const graphRef = ref<ReturnType<typeof useGraphEditor> | null>(null);

const scale = computed(() => graph?.scale.value ?? 1);
const panX = computed(() => graph?.panX.value ?? 40);
const panY = computed(() => graph?.panY.value ?? 40);
const selectedKnotId = computed(() => graph?.selectedKnotId.value ?? null);
const selectedKnot = computed(() => graph?.selectedKnot.value ?? null);

const selectedKnotSchema = computed(() => {
  const type = graph?.selectedKnot.value?.type;
  if (!type) return [];
  return knotCatalog.value.find((item) => item.kind === type)?.schema ?? [];
});

const canvasEl = ref<HTMLDivElement | null>(null);

let draggingId: string | null = null;
let dragOffset = { x: 0, y: 0 };
let panning = false;
let panStart = { x: 0, y: 0 };
const wireDrag = ref<{
  fromId: string;
  side: "in" | "out";
  index: number;
  from: { x: number; y: number };
  to: { x: number; y: number };
} | null>(null);

async function load() {
  try {
    const [wfBody, knotsBody] = await Promise.all([
      api.getWorkflow(props.id),
      api.listKnots(),
    ]);
    workflowMeta = wfBody.data;
    knotCatalog.value = knotsBody.data;
    workflowName.value = workflowMeta.name;
    graph = useGraphEditor(
      workflowMeta.knots,
      workflowMeta.connections,
      knotCatalog.value,
    );
    graphRef.value = graph;
    loaded.value = true;
  } catch (e) {
    errorMessage.value = e instanceof ApiError ? e.message : String(e);
  }
}

function canvasPoint(e: MouseEvent) {
  const g = graph!;
  const rect = canvasEl.value!.getBoundingClientRect();
  return {
    x: (e.clientX - rect.left - g.panX.value) / g.scale.value,
    y: (e.clientY - rect.top - g.panY.value) / g.scale.value,
  };
}

function onCanvasDrop(e: DragEvent) {
  e.preventDefault();
  const type = e.dataTransfer?.getData("text/kind");
  if (!type || !graph) return;
  const p = canvasPoint(e as unknown as MouseEvent);
  graph.addKnot(type, p.x, p.y);
}

function onKnotMouseDown(e: MouseEvent, id: string) {
  const g = graph!;
  g.selectKnot(id);
  const k = g.knots.find((k) => k.id === id)!;
  draggingId = id;
  const p = canvasPoint(e);
  dragOffset = { x: p.x - k.x, y: p.y - k.y };
}

function onPortMouseDown(
  e: MouseEvent,
  id: string,
  side: "in" | "out",
  index: number,
) {
  e.stopPropagation();
  const g = graph!;
  const start = g.portPos(id, side, index);
  wireDrag.value = { fromId: id, side, index, from: start, to: start };
  g.selectConnection(null);
}

function onCanvasMouseDown(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (
    target === canvasEl.value ||
    target.classList.contains("wires") ||
    target.classList.contains("empty-canvas")
  ) {
    const g = graph!;
    panning = true;
    panStart = { x: e.clientX - g.panX.value, y: e.clientY - g.panY.value };
    graph?.selectKnot(null);
    graph?.selectConnection(null);
  }
}

function onWindowMouseMove(e: MouseEvent) {
  if (!graph) return;
  const g = graph;
  if (draggingId) {
    const k = g.knots.find((k) => k.id === draggingId);
    if (k) {
      const p = canvasPoint(e);
      k.x = p.x - dragOffset.x;
      k.y = p.y - dragOffset.y;
    }
  } else if (wireDrag.value) {
    const p = canvasPoint(e);
    if (wireDrag.value.side === "out") wireDrag.value.to = p;
    else {
      wireDrag.value.to = wireDrag.value.from;
      wireDrag.value.from = p;
    }
  } else if (panning) {
    g.panX.value = e.clientX - panStart.x;
    g.panY.value = e.clientY - panStart.y;
  }
}

function onWindowMouseUp(e: MouseEvent) {
  if (draggingId) {
    graph?.markDirty();
    draggingId = null;
  }
  if (wireDrag.value && graph) {
    const target = (e.target as HTMLElement).closest(
      ".port",
    ) as HTMLElement | null;
    if (target) {
      const otherId = target.dataset.id!;
      const otherSide = target.dataset.port as "in" | "out";
      const otherIndex = Number(target.dataset.portIndex ?? 0);
      const wd = wireDrag.value;
      if (otherSide !== wd.side) {
        if (wd.side === "out") {
          graph.addConnection(wd.fromId, wd.index, otherId, otherIndex);
        } else {
          graph.addConnection(otherId, otherIndex, wd.fromId, wd.index);
        }
      }
    }
    wireDrag.value = null;
  }
  panning = false;
}

function selectKnot(id: string) {
  graph!.selectKnot(id);
}

function onNameInput() {
  graph?.markDirty();
}
function knotGlyphTransform(
  p1: { x: number; y: number },
  p2: { x: number; y: number },
) {
  const mx = (p1.x + p2.x) / 2;
  const my = (p1.y + p2.y) / 2;
  return `translate(${mx}, ${my})`;
}

async function save() {
  if (!graph || !workflowMeta) return;
  const payload = {
    name: workflowName.value || "(untitled)",
    active: workflowMeta.active,
    knots: graph.toWireKnots(),
    connections: [...graph.connections],
  };
  try {
    const body = await api.updateWorkflow(props.id, payload);
    workflowMeta = body.data;
    graph.dirty.value = false;
    showToast("Workflow saved");
  } catch (e) {
    errorMessage.value = e instanceof ApiError ? e.message : String(e);
  }
}

function discard() {
  if (graph?.dirty.value && !confirm("Discard changes to this workflow?"))
    return;
  router.push({ name: "list" });
}

function beforeUnloadHandler(e: BeforeUnloadEvent) {
  if (graph?.dirty.value) {
    e.preventDefault();
    e.returnValue = "";
  }
}

onBeforeRouteLeave(() => {
  if (graph?.dirty.value) {
    return confirm("Discard changes to this workflow?");
  }
  return true;
});

onMounted(() => {
  load();
  window.addEventListener("mousemove", onWindowMouseMove);
  window.addEventListener("mouseup", onWindowMouseUp);
  window.addEventListener("beforeunload", beforeUnloadHandler);
});

onBeforeUnmount(() => {
  window.removeEventListener("mousemove", onWindowMouseMove);
  window.removeEventListener("mouseup", onWindowMouseUp);
  window.removeEventListener("beforeunload", beforeUnloadHandler);
});

function onWheel(e: WheelEvent) {
  e.preventDefault();
  graph?.setZoom(graph.scale.value - e.deltaY * 0.001);
}

function onDragStart(e: DragEvent, kind: string) {
  e.dataTransfer?.setData("text/kind", kind);
}

const outgoingConnections = computed(() => {
  const id = graph?.selectedKnot.value?.id;
  if (!id || !graph) return [];
  return graph.connections.filter((c) => c.from === id);
});

const incomingConnections = computed(() => {
  const id = graph?.selectedKnot.value?.id;
  if (!id || !graph) return [];
  return graph.connections.filter((c) => c.to === id);
});

const selectedConnectionIdx = computed(
  () => graph?.selectedConnection.value ?? null,
);

const selectedConnectionDetail = computed(() => {
  const idx = selectedConnectionIdx.value;
  if (idx === null || !graph) return null;
  return graph.connections[idx] ?? null;
});

function cutSelectedConnection() {
  const idx = selectedConnectionIdx.value;
  if (idx === null || !graph) return;
  graph.removeConnection(idx);
  graph.selectConnection(null);
  showToast("Connection cut");
}

function onWireClick(idx: number) {
  graph?.selectConnection(idx);
}
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
        @input="onNameInput"
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

  <div v-if="loaded && graphRef" class="editor-shell">
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

    <div class="canvas-wrap">
      <div class="canvas-toolbar">
        <button class="btn btn-small" @click="graphRef?.setZoom(scale - 0.1)">
          &minus;
        </button>
        <button class="btn btn-small" @click="graphRef?.resetView()">
          Reset view
        </button>
        <button class="btn btn-small" @click="graphRef?.setZoom(scale + 0.1)">
          +
        </button>
      </div>

      <div
        ref="canvasEl"
        class="canvas"
        :class="{ panning, connecting: !!wireDrag }"
        @dragover.prevent
        @drop="onCanvasDrop"
        @mousedown="onCanvasMouseDown"
        @wheel="onWheel"
      >
        <svg class="wires">
          <g :transform="`translate(${panX}, ${panY}) scale(${scale})`">
            <path
              v-for="(c, idx) in graphRef.connections"
              :key="idx"
              :d="
                graphRef.wirePath(
                  graphRef.portPos(c.from, 'out', c.fromOutput),
                  graphRef.portPos(c.to, 'in', c.toInput),
                )
              "
              fill="none"
              :stroke="selectedConnectionIdx === idx ? '#e0b04a' : '#d4a13d'"
              :stroke-width="selectedConnectionIdx === idx ? 2.2 : 1.6"
              stroke-dasharray="8 4"
              stroke-opacity="0.75"
              :style="{ pointerEvents: 'stroke', cursor: 'pointer' }"
              @click.stop="onWireClick(idx)"
            />
            <g
              v-for="(c, idx) in graphRef.connections"
              :key="`knot-${idx}`"
              :transform="
                knotGlyphTransform(
                  graphRef.portPos(c.from, 'out', c.fromOutput),
                  graphRef.portPos(c.to, 'in', c.toInput),
                )
              "
              :opacity="selectedConnectionIdx === idx ? 1 : 0.85"
            >
              <path
                d="M -7 0 A 4 4 0 1 1 1 0 A 4 4 0 1 1 -7 0 M -3 -3.2 A 4 4 0 1 1 5 -3.2 A 4 4 0 1 1 -3 -3.2"
                fill="none"
                :stroke="selectedConnectionIdx === idx ? '#e0b04a' : '#d4a13d'"
                stroke-width="1.4"
                stroke-opacity="0.9"
              />
            </g>
          </g>
        </svg>

        <div
          class="knot-layer"
          :style="{
            transform: `translate(${panX}px, ${panY}px) scale(${scale})`,
          }"
        >
          <div
            v-for="k in graphRef.knots"
            :key="k.id"
            class="knot"
            :class="{
              selected: k.id === selectedKnotId,
              dragging: draggingId === k.id,
            }"
            :style="{
              left: k.x + 'px',
              top: k.y + 'px',
              height: graphRef.nodeHeight(k.type) + 'px',
            }"
            @mousedown="onKnotMouseDown($event, k.id)"
          >
            <div class="knot-head">
              <span class="knot-kind">{{ k.type }}</span>
              <button
                class="knot-remove"
                title="remove knot"
                @click.stop="graphRef.removeKnot(k.id)"
              >
                &times;
              </button>
            </div>
            <div class="knot-body">
              <div class="muted" style="font-size: 10px">{{ k.id }}</div>
            </div>
            <div
              v-for="pi in graphRef.inputCount(k.type)"
              :key="`in-${pi}`"
              class="port port-in"
              :data-port="'in'"
              :data-port-index="pi - 1"
              :data-id="k.id"
              :style="{ top: graphRef.portOffsetY(k.id, 'in', pi - 1) + 'px' }"
              @mousedown="onPortMouseDown($event, k.id, 'in', pi - 1)"
            ></div>
            <div
              v-for="po in graphRef.outputCount(k.type)"
              :key="`out-${po}`"
              class="port port-out"
              :data-port="'out'"
              :data-port-index="po - 1"
              :data-id="k.id"
              :style="{ top: graphRef.portOffsetY(k.id, 'out', po - 1) + 'px' }"
              @mousedown="onPortMouseDown($event, k.id, 'out', po - 1)"
              @click.stop="selectKnot(k.id)"
            ></div>
          </div>
        </div>

        <div v-if="graphRef.knots.length === 0" class="empty-canvas">
          drag a knot from the catalog to begin
        </div>
      </div>

      <div class="canvas-zoom">
        <span>{{ Math.round(scale * 100) }}%</span>
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
              @click="cutSelectedConnection"
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
          @change="graphRef?.markDirty()"
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
            <button
              @click="
                graphRef.removeConnection(graphRef.connections.indexOf(c))
              "
            >
              &times;
            </button>
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
