<script setup lang="ts">
// One line of a tab Rincian (Neraca, Laba Rugi): its amount, where it comes
// from, and the parts that add up to it.
import type { LinePart } from "../balance-sheet";
import { formatDate, formatSheetNumber } from "../store/koperasi";

const props = defineProps<{
  number: number;
  label: string;
  amount: number;
  source?: string;
  parts?: LinePart[];
}>();

const hasDates = () => (props.parts ?? []).some((part) => part.date);

// A single part that only repeats the line needs no table.
const showParts = () => (props.parts ?? []).length > 1 || hasDates();
</script>

<template>
  <article class="breakdown-line">
    <header>
      <span>{{ number }}. {{ label }}</span>
      <strong class="num-cell">{{ formatSheetNumber(amount) }}</strong>
    </header>
    <p v-if="source" class="breakdown-line__source">{{ source }}</p>
    <table v-if="showParts()" class="breakdown-line__parts">
      <tbody>
        <tr v-for="(part, index) in parts" :key="index">
          <td v-if="hasDates()" class="breakdown-line__date">
            {{ part.date ? formatDate(part.date) : "" }}
          </td>
          <td>
            {{ part.label }}
            <small v-if="part.category">{{ part.category }}</small>
          </td>
          <td class="num-cell">{{ formatSheetNumber(part.amount) }}</td>
        </tr>
      </tbody>
    </table>
  </article>
</template>

<style scoped>
.breakdown-line {
  padding: 8px 10px;
  border: 1px solid var(--line);
  border-radius: 6px;
}
.breakdown-line > header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  font-size: 12px;
}
.breakdown-line__source {
  margin: 3px 0 0;
  color: var(--soft);
  font-size: 11px;
}
.breakdown-line__parts {
  width: 100%;
  margin-top: 6px;
  border-collapse: collapse;
  font-size: 11.5px;
}
.breakdown-line__parts td {
  padding: 3px 0;
  border-top: 1px dashed var(--line);
  vertical-align: top;
}
.breakdown-line__parts td + td {
  padding-left: 10px;
}
.breakdown-line__parts small {
  display: block;
  color: var(--soft);
  font-size: 10.5px;
}
.breakdown-line__date {
  width: 82px;
  white-space: nowrap;
  color: var(--muted);
}
</style>
