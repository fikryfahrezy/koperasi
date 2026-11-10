<script setup lang="ts">
import { ref } from "vue";
import { formatSheetNumber } from "../store/koperasi";

// A Neraca amount edited in its cell: shown as in the sheet, typed as plain
// digits (a leading "-" for a negative). Enter or leaving the cell saves when
// the amount changed; Esc puts it back.
const props = defineProps<{ value: number; label: string }>();
const emit = defineEmits<{ commit: [value: number] }>();
const editing = ref(false);
const draft = ref("");

function start(event: FocusEvent) {
  editing.value = true;
  draft.value = props.value ? String(props.value) : "";
  requestAnimationFrame(() => (event.target as HTMLInputElement).select());
}

function finish() {
  if (!editing.value) return;
  editing.value = false;
  const digits = draft.value.replace(/[^\d]/g, "");
  const value = (draft.value.trim().startsWith("-") ? -1 : 1) * Number(digits);
  if (value !== props.value) emit("commit", value);
}

function cancel(event: KeyboardEvent) {
  editing.value = false;
  (event.target as HTMLInputElement).blur();
}
</script>

<template>
  <input
    class="sheet-amount"
    inputmode="numeric"
    :value="editing ? draft : formatSheetNumber(value)"
    :aria-label="label"
    :title="`${label}: klik untuk mengubah`"
    @focus="start"
    @input="draft = ($event.target as HTMLInputElement).value"
    @keydown.enter.prevent="($event.target as HTMLInputElement).blur()"
    @keydown.esc.prevent="cancel"
    @blur="finish"
  />
</template>

<style scoped>
.sheet-amount {
  width: 100%;
  min-width: 0;
  padding: 0;
  border: 0;
  border-bottom: 1px dashed var(--line-strong);
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: right;
  cursor: text;
}
.sheet-amount:hover {
  border-bottom-color: var(--green);
}
.sheet-amount:focus {
  outline: 2px solid var(--green);
  outline-offset: 1px;
  border-bottom-color: transparent;
  background: #fff;
}
</style>
