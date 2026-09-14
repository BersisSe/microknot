import { createApp } from "vue";
import App from "./App.vue";
import { router } from "./router";
import "./theme.css";
import "@vue-flow/core/dist/style.css";
import "@vue-flow/core/dist/theme-default.css";
import "@vue-flow/controls/dist/style.css";

createApp(App).use(router).mount("#app");
