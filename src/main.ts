import { createApp } from "vue";
import { VueQueryPlugin } from "@tanstack/vue-query";
import { queryClient } from "./query-client";
import App from "./App.vue";
import "./App.css";
import { router } from "./router";
import { reportRuntimeError } from "./runtime-error";
import { i18n } from "./i18n";
import { tableNavigation } from "./directives/tableNavigation";

window.addEventListener("error", (event) => {
  reportRuntimeError(event.error ?? event.message, "Unhandled window error");
});

window.addEventListener("unhandledrejection", (event) => {
  reportRuntimeError(event.reason, "Unhandled promise rejection");
});

const app = createApp(App);
app.directive("table-navigation", tableNavigation);

app.config.errorHandler = (error, instance, info) => {
  const componentName = instance?.$options.name ?? "UnknownComponent";

  reportRuntimeError(error, `Vue error in ${componentName}: ${info}`);
};

app.use(VueQueryPlugin, { queryClient }).use(i18n).use(router).mount("#app");
