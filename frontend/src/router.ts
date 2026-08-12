import { createRouter, createWebHistory } from "vue-router";
import WorkflowList from "./views/WorkflowList.vue";
import WorkflowEditor from "./views/WorkflowEditor.vue";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", name: "list", component: WorkflowList },
    { path: "/editor/:id", name: "editor", component: WorkflowEditor, props: true },
  ],
});
