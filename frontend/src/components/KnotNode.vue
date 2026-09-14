<script setup lang="ts">
import { computed } from "vue";
import { Handle, Position } from "@vue-flow/core";
import type { Knot, KnotDescriptor } from "../types";

const HEADER_H = 30;
const PORT_STEP = 24;
const BODY_PAD = 8;

const props = defineProps<{
  id: string;
  data: {
    knot: Knot;
    descriptor?: KnotDescriptor;
    onRemove: (id: string) => void;
  };
}>();

const inputCount = computed(() => props.data.descriptor?.inputCount ?? 1);
const outputCount = computed(() => props.data.descriptor?.outputCount ?? 1);
const maxPorts = computed(() =>
  Math.max(inputCount.value, outputCount.value, 1),
);
const height = computed(
  () => HEADER_H + BODY_PAD + maxPorts.value * PORT_STEP,
);

// Mirrors the old editor's port geometry: distribute this side's ports evenly
// over the body area, which is sized by whichever side has more ports.
function handleStyle(side: "in" | "out", index: number) {
  const count = side === "in" ? inputCount.value : outputCount.value;
  const n = Math.max(count, 1);
  const body = maxPorts.value * PORT_STEP;
  const top = HEADER_H + BODY_PAD / 2 + (index + 0.5) * (body / n);
  return { top: `${top}px` };
}

function remove() {
  props.data.onRemove(props.id);
}
</script>

<template>
  <div class="knot" :style="{ height: height + 'px' }">
    <div class="knot-head">
      <span class="knot-kind">{{ data.knot.type }}</span>
      <button class="knot-remove" title="remove knot" @click.stop="remove">
        &times;
      </button>
    </div>
    <div class="knot-body">
      <div class="muted" style="font-size: 10px">{{ data.knot.id }}</div>
    </div>
    <Handle
      v-for="i in Math.max(inputCount, 1)"
      :key="`in-${i}`"
      type="target"
      :position="Position.Left"
      :id="String(i - 1)"
      class="port port-in"
      :style="handleStyle('in', i - 1)"
    />
    <Handle
      v-for="o in Math.max(outputCount, 1)"
      :key="`out-${o}`"
      type="source"
      :position="Position.Right"
      :id="String(o - 1)"
      class="port port-out"
      :style="handleStyle('out', o - 1)"
    />
  </div>
</template>
