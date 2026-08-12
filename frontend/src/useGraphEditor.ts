import { ref, reactive, computed } from "vue";
import type { Knot, Connection, LaidOutKnot, KnotDescriptor } from "./types";

const KNOT_W = 168;
const HEADER_H = 30;
const PORT_STEP = 24;
const BODY_PAD = 8;
const COL_WIDTH = 220;
const ROW_HEIGHT = 100;

// Auto-layout: left-to-right by dependency depth (topological-ish pass).
// Position is never persisted server-side, so this runs fresh every load.
function layoutKnots(knots: Knot[], connections: Connection[]): LaidOutKnot[] {
  const incoming: Record<string, number> = {};
  knots.forEach((k) => (incoming[k.id] = 0));
  connections.forEach((c) => {
    if (incoming[c.to] !== undefined) incoming[c.to] += 1;
  });

  const colOf: Record<string, number> = {};
  let frontier = knots.filter((k) => incoming[k.id] === 0).map((k) => k.id);
  frontier.forEach((id) => (colOf[id] = 0));

  let guard = 0;
  while (frontier.length && guard < 200) {
    guard += 1;
    const next: string[] = [];
    frontier.forEach((id) => {
      connections.forEach((c) => {
        if (c.from === id) {
          const candidate = (colOf[id] ?? 0) + 1;
          if (colOf[c.to] === undefined || colOf[c.to] < candidate) colOf[c.to] = candidate;
          if (!next.includes(c.to)) next.push(c.to);
        }
      });
    });
    frontier = next;
  }
  knots.forEach((k) => {
    if (colOf[k.id] === undefined) colOf[k.id] = 0;
  });

  const colCounts: Record<number, number> = {};
  return knots.map((k) => {
    const col = colOf[k.id];
    const row = colCounts[col] || 0;
    colCounts[col] = row + 1;
    return { ...k, x: 40 + col * COL_WIDTH, y: 40 + row * ROW_HEIGHT };
  });
}

export function useGraphEditor(
  initialKnots: Knot[],
  initialConnections: Connection[],
  descriptors: KnotDescriptor[],
) {
  const knots = reactive<LaidOutKnot[]>(layoutKnots(initialKnots, initialConnections));
  const connections = reactive<Connection[]>(initialConnections.map((c) => ({ ...c })));

  const counts = new Map(descriptors.map((d) => [d.kind, d]));

  const selectedKnotId = ref<string | null>(null);
  const selectedConnection = ref<number | null>(null);
  const dirty = ref(false);

  const scale = ref(1);
  const panX = ref(40);
  const panY = ref(40);

  let knotSeq = knots.reduce((max, k) => {
    const m = /^k(\d+)$/.exec(k.id);
    return m ? Math.max(max, parseInt(m[1], 10) + 1) : max;
  }, 1);

  const selectedKnot = computed(() => knots.find((k) => k.id === selectedKnotId.value) ?? null);

  function inputCount(type: string): number {
    return counts.get(type)?.inputCount ?? 1;
  }

  function outputCount(type: string): number {
    return counts.get(type)?.outputCount ?? 1;
  }

  function maxPorts(type: string): number {
    return Math.max(inputCount(type), outputCount(type), 1);
  }

  function nodeHeight(type: string): number {
    return HEADER_H + BODY_PAD + maxPorts(type) * PORT_STEP;
  }

  // Y offset of a port relative to the node's own top edge (for CSS placement).
  function portOffsetY(id: string, side: "in" | "out", index: number): number {
    const k = knots.find((k) => k.id === id);
    if (!k) return 0;
    const count = side === "out" ? outputCount(k.type) : inputCount(k.type);
    const n = Math.max(count, 1);
    const body = maxPorts(k.type) * PORT_STEP;
    return HEADER_H + BODY_PAD / 2 + (index + 0.5) * (body / n);
  }

  function markDirty() {
    dirty.value = true;
  }

  function selectKnot(id: string | null) {
    selectedKnotId.value = id;
    if (id !== null) selectedConnection.value = null;
  }

  function selectConnection(index: number | null) {
    selectedConnection.value = index;
    if (index !== null) selectedKnotId.value = null;
  }

  function addKnot(type: string, canvasX: number, canvasY: number) {
    const id = "k" + knotSeq++;
    knots.push({ id, type, params: {}, x: canvasX - KNOT_W / 2, y: canvasY - nodeHeight(type) / 2 });
    markDirty();
    selectKnot(id);
    return id;
  }

  function removeKnot(id: string) {
    const idx = knots.findIndex((k) => k.id === id);
    if (idx >= 0) knots.splice(idx, 1);
    for (let i = connections.length - 1; i >= 0; i--) {
      if (connections[i].from === id || connections[i].to === id) connections.splice(i, 1);
    }
    if (selectedKnotId.value === id) selectedKnotId.value = null;
    markDirty();
  }

  function addConnection(from: string, fromOutput: number, to: string, toInput: number) {
    if (from === to) return false;
    const fromKnot = knots.find((k) => k.id === from);
    const toKnot = knots.find((k) => k.id === to);
    if (!fromKnot || !toKnot) return false;
    if (fromOutput >= outputCount(fromKnot.type)) return false;
    if (toInput >= inputCount(toKnot.type)) return false;
    const exists = connections.some(
      (c) => c.from === from && c.fromOutput === fromOutput && c.to === to && c.toInput === toInput,
    );
    if (exists) return false;
    connections.push({ from, fromOutput, to, toInput });
    markDirty();
    selectConnection(null);
    return true;
  }

  function removeConnection(index: number) {
    connections.splice(index, 1);
    if (selectedConnection.value === index) selectedConnection.value = null;
    markDirty();
  }

  function portPos(id: string, side: "in" | "out", index: number) {
    const k = knots.find((k) => k.id === id);
    if (!k) return { x: 0, y: 0 };
    return { x: k.x + (side === "out" ? KNOT_W : 0), y: k.y + portOffsetY(id, side, index) };
  }

  function wirePath(p1: { x: number; y: number }, p2: { x: number; y: number }) {
    const dx = Math.max(Math.abs(p2.x - p1.x) * 0.5, 40);
    return `M ${p1.x} ${p1.y} C ${p1.x + dx} ${p1.y}, ${p2.x - dx} ${p2.y}, ${p2.x} ${p2.y}`;
  }

  function setZoom(z: number) {
    scale.value = Math.min(2, Math.max(0.4, z));
  }

  function resetView() {
    scale.value = 1;
    panX.value = 40;
    panY.value = 40;
  }

  // Strip client-only x/y before sending to the server.
  function toWireKnots(): Knot[] {
    return knots.map(({ x: _x, y: _y, ...rest }) => rest);
  }

  return {
    knots,
    connections,
    selectedKnotId,
    selectedKnot,
    selectedConnection,
    dirty,
    scale,
    panX,
    panY,
    inputCount,
    outputCount,
    nodeHeight,
    portOffsetY,
    markDirty,
    selectKnot,
    selectConnection,
    addKnot,
    removeKnot,
    addConnection,
    removeConnection,
    portPos,
    wirePath,
    setZoom,
    resetView,
    toWireKnots,
  };
}
