<script setup lang="ts">
import { ref } from "vue";
import { FileSpreadsheet } from "lucide-vue-next";

import {
  type ExportFilter,
  type ExportKind,
  useKoperasiStore,
} from "../store/koperasi";

// Saves the page's year as .xlsx in the layout of its workbook, with formulas,
// limited to what the page's filters show.
const props = defineProps<{ kind: ExportKind; filter?: ExportFilter }>();

const { exportXlsx } = useKoperasiStore();
const exporting = ref(false);

async function handleExport() {
  if (exporting.value) return;
  exporting.value = true;

  try {
    await exportXlsx(props.kind, props.filter);
  } finally {
    exporting.value = false;
  }
}
</script>

<template>
  <button
    class="button button--secondary"
    type="button"
    :disabled="exporting"
    @click="handleExport"
  >
    <FileSpreadsheet :size="16" />
    {{ exporting ? "Mengekspor…" : "Ekspor Excel" }}
  </button>
</template>
