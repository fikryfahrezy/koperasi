<script setup lang="ts">
import { computed, ref, watch } from "vue";
import UiSelect, { type SelectValue } from "./UiSelect.vue";

const props = withDefaults(
  defineProps<{
    required?: boolean;
    disabled?: boolean;
    ariaLabel?: string;
  }>(),
  { required: false, disabled: false, ariaLabel: "Tanggal" },
);
const model = defineModel<string>({ required: true });
const day = ref<SelectValue>("");
const month = ref<SelectValue>("");
const year = ref<SelectValue>("");

const daysInMonth = computed(() => {
  const selectedYear = Number(year.value) || 2000;
  const leapYear =
    selectedYear % 4 === 0 &&
    (selectedYear % 100 !== 0 || selectedYear % 400 === 0);
  const days = [31, leapYear ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  return days[Number(month.value) - 1] ?? 31;
});
const dayOptions = computed(() => numberOptions(1, daysInMonth.value));
const monthOptions = numberOptions(1, 12);
const yearOptions = computed(() => {
  const currentYear = new Date().getFullYear();
  // Keep historical dates already in the model available as well.
  return numberOptions(
    Math.min(1900, Number(year.value) || 1900),
    Math.max(currentYear + 100, Number(year.value)),
    4,
  );
});

function numberOptions(first: number, last: number, width = 2) {
  return Array.from({ length: last - first + 1 }, (_, index) => {
    const value = first + index;
    return { value, label: String(value).padStart(width, "0") };
  });
}

function isoValue() {
  if (!day.value || !month.value || !year.value) return "";
  return `${String(year.value).padStart(4, "0")}-${String(month.value).padStart(2, "0")}-${String(day.value).padStart(2, "0")}`;
}

function updatePart(part: "day" | "month" | "year", value: SelectValue) {
  if (part === "day") day.value = value;
  if (part === "month") month.value = value;
  if (part === "year") year.value = value;
  // Ask for a new day instead of silently changing the selected date.
  if (Number(day.value) > daysInMonth.value) day.value = "";
  model.value = isoValue();
}

watch(
  model,
  (value) => {
    // Preserve a partial selection when its empty value is echoed by v-model.
    if (value === isoValue()) return;
    const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
    year.value = match ? Number(match[1]) : "";
    month.value = match ? Number(match[2]) : "";
    day.value = match ? Number(match[3]) : "";
  },
  { immediate: true },
);
</script>

<template>
  <div class="date-input" role="group" :aria-label="ariaLabel">
    <UiSelect
      :model-value="day"
      :options="dayOptions"
      :aria-label="`${ariaLabel}: hari`"
      placeholder="DD"
      :required="props.required"
      :disabled="props.disabled"
      @update:model-value="updatePart('day', $event)"
    />
    <span aria-hidden="true">/</span>
    <UiSelect
      :model-value="month"
      :options="monthOptions"
      :aria-label="`${ariaLabel}: bulan`"
      placeholder="MM"
      :required="props.required"
      :disabled="props.disabled"
      @update:model-value="updatePart('month', $event)"
    />
    <span aria-hidden="true">/</span>
    <UiSelect
      :model-value="year"
      :options="yearOptions"
      :aria-label="`${ariaLabel}: tahun`"
      placeholder="YYYY"
      :required="props.required"
      :disabled="props.disabled"
      @update:model-value="updatePart('year', $event)"
    />
  </div>
</template>

<style scoped>
.date-input {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr) auto minmax(
      0,
      1.4fr
    );
  align-items: center;
  gap: 8px;
}
.date-input > span {
  color: var(--soft);
}
</style>
