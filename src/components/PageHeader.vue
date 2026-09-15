<script setup lang="ts">
import { ref } from "vue";

const props = defineProps<{
  title: string;
  refresh?: () => Promise<unknown> | unknown;
}>();

const refreshing = ref(false);

async function handleRefresh() {
  if (!props.refresh || refreshing.value) return;
  refreshing.value = true;
  try {
    await props.refresh();
  } finally {
    refreshing.value = false;
  }
}
</script>

<template>
  <header class="page-heading">
    <h1>{{ title }}</h1>
    <div v-if="refresh || $slots.actions" class="page-heading__actions">
      <button
        v-if="refresh"
        class="button button--secondary"
        type="button"
        :disabled="refreshing"
        :aria-label="refreshing ? 'Memuat ulang data' : 'Muat ulang data'"
        @click="handleRefresh"
      >
        {{ refreshing ? "Memuat ulang…" : "Muat ulang" }}
      </button>
      <slot name="actions" />
    </div>
  </header>
</template>
