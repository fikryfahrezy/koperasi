<script setup lang="ts" generic="T extends string | boolean">
// Views of one page, in its title (Buku Kas: Buku kas harian | Riwayat hapus).
// The view shown is the page's h1; the others are buttons to it. Not
// role="tab": that role is the workspace tabs'.
defineProps<{
  label: string;
  tabs: { value: T; label: string; count?: number }[];
}>();

const model = defineModel<T>({ required: true });
</script>

<template>
  <nav class="page-tabs" :aria-label="label">
    <template v-for="tab in tabs" :key="tab.label">
      <h1 v-if="model === tab.value" class="page-tabs__tab is-active">
        {{ tab.label }}
      </h1>
      <button
        v-else
        class="page-tabs__tab"
        type="button"
        @click="model = tab.value"
      >
        {{ tab.label }}
        <span v-if="tab.count" class="page-tabs__count">{{ tab.count }}</span>
      </button>
    </template>
  </nav>
</template>

<style scoped>
/* A segmented control: the view shown is raised, the others are buttons. */
.page-tabs {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 3px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface-soft);
}
.page-tabs__tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin: 0;
  padding: 5px 12px;
  border: 1px solid transparent;
  border-radius: 6px;
  color: var(--muted);
  background: transparent;
  font-size: 15px;
  font-weight: 700;
  line-height: 1.15;
  letter-spacing: -0.03em;
  cursor: pointer;
}
.page-tabs__tab:hover {
  color: var(--green);
  background: var(--green-soft);
}
.page-tabs__tab:focus-visible {
  outline: 2px solid var(--green);
  outline-offset: 1px;
}
.page-tabs__tab.is-active {
  border-color: var(--line-strong);
  color: var(--ink);
  background: var(--surface);
  box-shadow: 0 1px 2px rgb(0 0 0 / 8%);
  cursor: default;
}
.page-tabs__count {
  min-width: 18px;
  padding: 1px 5px;
  border-radius: 9px;
  color: var(--red);
  background: var(--red-soft);
  font-size: 10.5px;
  text-align: center;
}
</style>
