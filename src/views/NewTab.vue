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
