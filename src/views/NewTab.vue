<script setup lang="ts">
import { LayoutGrid } from "lucide-vue-next";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
</script>

<template>
  <section class="new-tab-page">
    <span><LayoutGrid :size="24" /></span>
    <h1>{{ t("workspaceTabs.newTab") }}</h1>
    <p>{{ t("workspaceTabs.emptyDescription") }}</p>
  </section>
</template>

<style scoped>
.new-tab-page {
  min-height: 100%;
  display: grid;
  align-content: center;
  justify-items: center;
  padding-bottom: 12vh;
  color: var(--muted);
  text-align: center;
}
.new-tab-page > span {
  width: 54px;
  height: 54px;
  display: grid;
  place-items: center;
  margin-bottom: 16px;
  color: var(--green);
  background: var(--green-soft);
}
.new-tab-page h1 {
  margin-bottom: 6px;
  color: var(--ink);
  font-size: 20px;
}
.new-tab-page p {
  margin: 0;
  font-size: 13px;
}
</style>
