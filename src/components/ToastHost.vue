<script setup lang="ts">
import { CheckCircle2, FolderOpen, Info, TriangleAlert } from "lucide-vue-next";
import { useKoperasiStore } from "../store/koperasi";

const { toasts, openExportFolder } = useKoperasiStore();
</script>

<template>
  <div class="toast-stack" aria-live="polite">
    <article
      v-for="toast in toasts"
      :key="toast.id"
      :class="['toast-card', `toast-card--${toast.tone}`]"
    >
      <CheckCircle2 v-if="toast.tone === 'success'" :size="20" />
      <TriangleAlert v-else-if="toast.tone === 'warning'" :size="20" />
      <Info v-else :size="20" />
      <div>
        <strong>{{ toast.title }}</strong>
        <p>{{ toast.message }}</p>
        <button
          v-if="toast.exportPath"
          type="button"
          class="button button--secondary toast-folder-button"
          :disabled="toast.openingFolder"
          @click="openExportFolder(toast)"
        >
          <FolderOpen :size="14" aria-hidden="true" />
          {{ $t("notifications.openFolder") }}
        </button>
      </div>
    </article>
  </div>
</template>

<style scoped>
.toast-stack {
  position: fixed;
  z-index: 200;
  right: 24px;
  bottom: 24px;
  width: min(380px, calc(100vw - 32px));
  display: grid;
  gap: 9px;
}
.toast-card {
  display: grid;
  grid-template-columns: 21px 1fr;
  gap: 11px;
  padding: 14px 15px;
  border: 1px solid #b6ccd8;
  border-radius: 11px;
  color: var(--green);
  background: #fff;
}
.toast-card > div {
  min-width: 0;
  display: grid;
  gap: 3px;
}
.toast-card strong {
  color: var(--ink);
  font-size: 11px;
}
.toast-card p {
  overflow-wrap: anywhere;
  margin: 0;
  color: var(--muted);
  font-size: 9.5px;
  line-height: 1.45;
}
.toast-folder-button {
  justify-self: start;
  margin-top: 6px;
  padding: 6px 9px;
  font-size: 11px;
}
.toast-card--warning {
  color: var(--amber);
  border-color: #ebd0a6;
}
.toast-card--info {
  color: var(--blue);
  border-color: #c7d6f7;
}
</style>
