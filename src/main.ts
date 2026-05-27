import { createApp } from "vue";
import "./App.css";
import App from "./App.vue";
import { reportRuntimeError } from "./runtime-error";

window.addEventListener("error", (event) => {
	reportRuntimeError(event.error ?? event.message, "Unhandled window error");
});

window.addEventListener("unhandledrejection", (event) => {
	reportRuntimeError(event.reason, "Unhandled promise rejection");
});

const app = createApp(App);

app.config.errorHandler = (error, instance, info) => {
	const componentName = instance?.$options.name ?? "UnknownComponent";

	reportRuntimeError(error, `Vue error in ${componentName}: ${info}`);
};

app.mount("#app");
