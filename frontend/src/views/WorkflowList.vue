<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { api, ApiError } from "../api";
import { showToast } from "../toast";
import type { Workflow, KnotDescriptor, RunResult } from "../types";

const router = useRouter();

const workflows = ref<Workflow[]>([]);
const knots = ref<KnotDescriptor[]>([]);
const selectedId = ref<string | null>(null);
const runResult = ref<RunResult | null>(null);
const errorMessage = ref<string | null>(null);

const selected = computed(() => workflows.value.find((w) => w.id === selectedId.value) ?? null);

async function loadWorkflows() {
  try {
    const body = await api.listWorkflows();
    workflows.value = body.data;
    if (selectedId.value && !workflows.value.some((w) => w.id === selectedId.value)) {
      selectedId.value = null;
    }
    errorMessage.value = null;
  } catch (e) {
    errorMessage.value = e instanceof ApiError ? e.message : String(e);
  }
}

async function loadKnots() {
  try {
    const body = await api.listKnots();
    knots.value = body.data;
  } catch (e) {
    errorMessage.value = e instanceof ApiError ? e.message : String(e);
  }
}

async function selectWorkflow(id: string) {
  selectedId.value = id;
  runResult.value = null;
  try {
    const body = await api.getWorkflow(id);
    const idx = workflows.value.findIndex((w) => w.id === id);
    if (idx >= 0) workflows.value[idx] = body.data;
  } catch (e) {
    errorMessage.value = e instanceof ApiError ? e.message : String(e);
  }
}

async function createWorkflow() {
  try {
    const body = await api.createWorkflow({
      name: `New workflow ${new Date().toISOString().slice(0, 19).replace("T", " ")}`,
      active: false,
      knots: [
        { id: "a", type: "trigger.webhook", params: {} },
        { id: "b", type: "notify.log", params: { message: "Hello {{ $json.name }} from microknot" } },
      ],
      connections: [{ from: "a", fromOutput: 0, to: "b", toInput: 0 }],
    });
    showToast("Workflow created");
    router.push({ name: "editor", params: { id: body.data.id } });
  } catch (e) {
    errorMessage.value = e instanceof ApiError ? e.message : String(e);
  }
}

async function runWorkflow(id: string) {
  try {
    const body = await api.runWorkflow(id);
    runResult.value = body.data;
    showToast(`Run finished: ${body.data.status}`);
  } catch (e) {
    errorMessage.value = e instanceof ApiError ? e.message : String(e);
  }
}

async function deleteWorkflow(id: string, name: string) {
  if (!confirm(`Delete "${name}"? This cannot be undone.`)) return;
  try {
    await api.deleteWorkflow(id);
    showToast("Workflow deleted");
    if (selectedId.value === id) selectedId.value = null;
    await loadWorkflows();
  } catch (e) {
    errorMessage.value = e instanceof ApiError ? e.message : String(e);
  }
}

function editWorkflow(id: string) {
  router.push({ name: "editor", params: { id } });
}

onMounted(() => {
  loadWorkflows();
  loadKnots();
});
</script>

<template>
  <header class="topbar">
    <img src="../assets/knot.svg" alt="MicroKnot Logo" class="brand-img">
    <router-link to="/" class="brand" style="text-decoration: none;">microknot</router-link>
    <span class="tagline">local-first workflow engine</span>
    <div class="topbar-actions">
      <button class="btn btn-primary" @click="createWorkflow">New Workflow</button>
      <button class="btn" @click="loadWorkflows">Refresh</button>
    </div>
  </header>

  <div class="banner" :class="{ hidden: !errorMessage }">Error: {{ errorMessage }}</div>

  <main class="layout">
    <section class="panel">
      <div class="panel-header">
        <h2>Workflows</h2>
        <span class="muted">{{ workflows.length }} workflow{{ workflows.length === 1 ? "" : "s" }}</span>
      </div>
      <div class="workflow-list">
        <div v-if="workflows.length === 0" class="empty">No workflows yet. Create one to get started.</div>
        <div
          v-for="wf in workflows"
          :key="wf.id"
          class="workflow-row"
          :class="{ selected: wf.id === selectedId }"
          tabindex="0"
          @click="selectWorkflow(wf.id)"
          @keydown.enter="selectWorkflow(wf.id)"
        >
          <span class="wf-name">{{ wf.name || "(untitled)" }}</span>
          <span class="wf-id">{{ wf.id.slice(0, 8) }}</span>
          <span class="wf-knots">{{ wf.knots.length }} knot{{ wf.knots.length === 1 ? "" : "s" }}</span>
          <span class="wf-active"><span class="badge" :class="wf.active ? 'on' : 'off'">{{ wf.active ? "ON" : "OFF" }}</span></span>
        </div>
      </div>
    </section>

    <section class="panel">
      <div class="panel-header">
        <h2>Detail</h2>
        <span class="detail-actions" v-if="selected">
          <button class="btn btn-small" @click="runWorkflow(selected.id)">Run</button>
          <button class="btn btn-small btn-primary" @click="editWorkflow(selected.id)">Edit</button>
          <button class="btn btn-small btn-danger" @click="deleteWorkflow(selected.id, selected.name)">Delete</button>
        </span>
      </div>
      <div class="detail">
        <div v-if="!selected" class="empty">Select a workflow to inspect it.</div>
        <template v-else>
          <h3>{{ selected.name || "(untitled)" }}</h3>
          <pre>{{ JSON.stringify(selected, null, 2) }}</pre>
          <div v-if="runResult" class="run-results">
            Run status: {{ runResult.status }} · {{ runResult.steps?.length ?? 0 }} steps
          </div>
        </template>
      </div>
    </section>
  </main>

  <section class="panel knots-panel">
    <div class="panel-header"><h2>Knot catalog</h2></div>
    <div class="knots">
      <div v-if="knots.length === 0" class="empty">No knot types available.</div>
      <div v-for="k in knots" :key="k.kind" class="knot-tile">
        <span class="k-kind">{{ k.kind }}</span>
        <span class="k-desc">{{ k.name }}</span>
      </div>
    </div>
  </section>
</template>
