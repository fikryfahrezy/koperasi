<script setup lang="ts">
// A tab Rincian (Neraca, Laba Rugi): two sides, each in sections of lines
// with the parts that add up to them, like
// NERACA PENYESUAIAN of BUKU KAS HARIAN.
import type { LinePart } from "../balance-sheet";
import { formatSheetNumber } from "../store/koperasi";
import BreakdownLine from "./BreakdownLine.vue";

export interface BreakdownSection {
  /** Empty for a side of one section. */
  title: string;
  lines: {
    label: string;
    amount: number;
    parts?: LinePart[];
  }[];
  /** Empty to leave the section without a total line. */
  totalLabel: string;
  total: number;
}

export interface BreakdownSide {
  title: string;
  sections: BreakdownSection[];
  totalLabel: string;
  total: number;
}

defineProps<{ title: string; position: string; sides: BreakdownSide[] }>();
</script>

<template>
  <section class="panel breakdown">
    <header class="breakdown__title">
      <strong>{{ title }}</strong>
      <u>POSISI {{ position }}</u>
    </header>

    <div v-for="side in sides" :key="side.title" class="breakdown__side">
      <h2>{{ side.title }}</h2>
      <section
        v-for="section in side.sections"
        :key="section.title"
        class="breakdown__section"
      >
        <h3 v-if="section.title">{{ section.title }}</h3>
        <p v-if="!section.lines.length" class="breakdown__empty">
          Belum ada pos.
        </p>
        <BreakdownLine
          v-for="(line, index) in section.lines"
          :key="index"
          :number="index + 1"
          :label="line.label || section.title"
          :amount="line.amount"
          :parts="line.parts"
        />
        <footer v-if="section.totalLabel" class="breakdown__total">
          <span>{{ section.totalLabel }}</span>
          <strong class="num-cell">{{
            formatSheetNumber(section.total)
          }}</strong>
        </footer>
      </section>
      <footer class="breakdown__total breakdown__total--grand">
        <span>{{ side.totalLabel }}</span>
        <strong class="num-cell">{{ formatSheetNumber(side.total) }}</strong>
      </footer>
    </div>
  </section>
</template>

<style scoped>
.breakdown {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px 28px;
  padding: 18px 20px;
}
.breakdown__title {
  display: grid;
  grid-column: 1 / -1;
  justify-items: center;
  gap: 2px;
  font-size: 12px;
}
.breakdown__side {
  display: grid;
  align-content: start;
  gap: 14px;
  min-width: 0;
}
.breakdown h2 {
  margin: 0;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--line-strong);
  font-size: 12px;
  letter-spacing: 0.04em;
}
.breakdown h3 {
  margin: 0 0 6px;
  font-size: 12px;
  text-decoration: underline;
}
.breakdown__section {
  display: grid;
  gap: 8px;
}
.breakdown__total {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  font-size: 12px;
}
.breakdown__empty {
  margin: 0;
  color: var(--soft);
  font-size: 11px;
}
.breakdown__total {
  padding: 0 10px;
  font-weight: 700;
}
.breakdown__total--grand {
  padding-top: 6px;
  border-top: 1px solid var(--line-strong);
}
@media (max-width: 900px) {
  .breakdown {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
