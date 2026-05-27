<script setup lang="ts">
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { ref } from "vue";

type AvailableUpdate = NonNullable<Awaited<ReturnType<typeof check>>>;

const isDev = import.meta.env.DEV;
const isCheckingForUpdates = ref(false);
const isInstallingUpdate = ref(false);
const statusMessage = ref(
  isDev
    ? "Updater is disabled while running in dev mode."
    : "Check for updates when you are ready.",
);
const availableUpdate = ref<AvailableUpdate | null>(null);

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
      console.error("Update install failed", error);
    }
  } finally {
    isInstallingUpdate.value = false;
  }
};
</script>

<template>
  <div class="min-h-screen bg-stone-950 px-6 py-16 text-stone-50">
    <div class="mx-auto flex max-w-xl items-center justify-center">
      <section class="w-full rounded-3xl border border-white/10 bg-white/5 p-8 shadow-2xl shadow-black/30 backdrop-blur">
        <p class="text-sm uppercase tracking-[0.3em] text-emerald-300/80">TemplateW7</p>
        <h1 class="mt-4 text-3xl font-semibold text-white">App updates</h1>
        <p class="mt-3 text-sm leading-6 text-stone-300">
          Check for a newer signed release and choose when to install it.
        </p>

        <div class="mt-8 rounded-2xl border border-white/10 bg-black/20 p-4 text-sm text-stone-200">
          {{ statusMessage }}
        </div>

        <div class="mt-6 flex flex-wrap gap-3">
          <button
            class="rounded-full bg-emerald-400 px-5 py-3 text-sm font-medium text-stone-950 transition hover:bg-emerald-300 disabled:cursor-not-allowed disabled:bg-emerald-900 disabled:text-stone-400"
            type="button"
            :disabled="isDev || isCheckingForUpdates || isInstallingUpdate"
            @click="checkForUpdates"
          >
            {{ isCheckingForUpdates ? 'Checking...' : 'Check for updates' }}
          </button>

          <button
            v-if="availableUpdate"
            class="rounded-full border border-white/15 px-5 py-3 text-sm font-medium text-white transition hover:border-white/30 hover:bg-white/5 disabled:cursor-not-allowed disabled:border-white/10 disabled:text-stone-500"
            type="button"
            :disabled="isInstallingUpdate || isCheckingForUpdates"
            @click="installUpdate"
          >
            {{ isInstallingUpdate ? 'Installing...' : `Install ${availableUpdate.version}` }}
          </button>
        </div>

        <p v-if="availableUpdate" class="mt-4 text-sm text-stone-400">
          Update {{ availableUpdate.version }} is ready to install.
        </p>
      </section>
    </div>
  </div>
</template>
