<script setup lang="ts">
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { ref, shallowRef } from "vue";
import { clearRuntimeError, reportRuntimeError, runtimeError } from "./runtime-error";

type AvailableUpdate = NonNullable<Awaited<ReturnType<typeof check>>>;

const isDev = import.meta.env.DEV;
const isCheckingForUpdates = ref(false);
const isInstallingUpdate = ref(false);
const statusMessage = ref(
  isDev
    ? "Updater is disabled while running in dev mode."
    : "Check for updates when you are ready.",
);
const availableUpdate = shallowRef<AvailableUpdate | null>(null);

const isOfflineError = (error: unknown) => {
  if (!error || typeof error !== "object") {
    return false;
  }

  const message = "message" in error ? String(error.message).toLowerCase() : "";

  return [
    "network",
    "internet",
    "dns",
    "offline",
    "timed out",
    "timeout",
    "failed to fetch",
    "connection",
  ].some((fragment) => message.includes(fragment));
};

const checkForUpdates = async () => {
  if (isDev || isCheckingForUpdates.value || isInstallingUpdate.value) {
    return;
  }

  if (typeof navigator !== "undefined" && !navigator.onLine) {
    statusMessage.value = "No internet connection. Reconnect and try again.";
    return;
  }

  isCheckingForUpdates.value = true;
  availableUpdate.value = null;
  statusMessage.value = "Checking for updates...";

  try {
    const update = await check();

    if (!update) {
      statusMessage.value = "You already have the latest version.";
      return;
    }

    availableUpdate.value = update;
    statusMessage.value = `Version ${update.version} is available.`;
  } catch (error) {
    if (isOfflineError(error)) {
      statusMessage.value = "No internet connection. Reconnect and try again.";
      return;
    }

    statusMessage.value = "Failed to check for updates.";
    reportRuntimeError(error, "Update check failed");
    console.error("Update check failed", error);
  } finally {
    isCheckingForUpdates.value = false;
  }
};

const installUpdate = async () => {
  if (!availableUpdate.value || isInstallingUpdate.value || isCheckingForUpdates.value) {
    return;
  }

  if (typeof navigator !== "undefined" && !navigator.onLine) {
    statusMessage.value = "No internet connection. Reconnect and try again.";
    return;
  }

  isInstallingUpdate.value = true;
  statusMessage.value = `Installing version ${availableUpdate.value.version}...`;

  try {
    await availableUpdate.value.downloadAndInstall();
    statusMessage.value = "Update installed. Restarting app...";
    await relaunch();
  } catch (error) {
    if (isOfflineError(error)) {
      statusMessage.value = "Connection was interrupted. Reconnect and try again.";
    } else {
      statusMessage.value = "Failed to install the update.";
      reportRuntimeError(error, "Update install failed");
      console.error("Update install failed", error);
    }
  } finally {
    isInstallingUpdate.value = false;
  }
};
</script>

<template>
  <div class="app-shell">
    <div class="app-shell__inner">
      <section class="update-card">
        <div v-if="runtimeError" class="error-panel">
          <div class="error-panel__header">
            <div>
              <p class="error-panel__eyebrow">Runtime error</p>
              <p class="error-panel__description">Bundled app errors are mirrored here when the webview console is not visible.</p>
            </div>
            <button
              class="error-panel__dismiss"
              type="button"
              @click="clearRuntimeError"
            >
              Dismiss
            </button>
          </div>

          <pre class="error-panel__body">{{ runtimeError }}</pre>
        </div>

        <p class="update-card__eyebrow">TemplateW7 #0.6.9</p>
        <h1 class="update-card__title">App updates</h1>
        <p class="update-card__description">
          Check for a newer signed release and choose when to install it.
        </p>

        <div class="status-panel">
          {{ statusMessage }}
        </div>

        <div class="actions-row">
          <button
            class="button button--primary"
            type="button"
            :disabled="isDev || isCheckingForUpdates || isInstallingUpdate"
            @click="checkForUpdates"
          >
            {{ isCheckingForUpdates ? 'Checking...' : 'Check for updates' }}
          </button>

          <button
            v-if="availableUpdate"
            class="button button--secondary"
            type="button"
            :disabled="isInstallingUpdate || isCheckingForUpdates"
            @click="installUpdate"
          >
            {{ isInstallingUpdate ? 'Installing...' : `Install ${availableUpdate.version}` }}
          </button>
        </div>

        <p v-if="availableUpdate" class="update-card__hint">
          Update {{ availableUpdate.version }} is ready to install.
        </p>
      </section>
    </div>
  </div>
</template>
