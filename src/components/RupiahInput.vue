<script setup lang="ts">
import { nextTick, onMounted, ref, watch } from "vue";
import { formatCurrency } from "../store/koperasi";

const props = defineProps<{
  min?: number;
  minMessage?: string;
}>();
const model = defineModel<number>({ required: true });
const input = ref<HTMLInputElement>();
const displayValue = ref(formatValue(model.value));

function formatValue(value: number) {
  return Number.isFinite(value) ? formatCurrency(value) : "";
}

function syncValidity() {
  const minimum = props.min;
  const belowMinimum =
    minimum !== undefined &&
    Number.isFinite(model.value) &&
    model.value < minimum;

  input.value?.setCustomValidity(
    belowMinimum
      ? (props.minMessage ?? `Nilai minimal ${formatCurrency(minimum)}.`)
      : "",
  );
}

function handleInput(event: Event) {
  const element = event.target as HTMLInputElement;
  const digits = element.value.replace(/\D/g, "");

  model.value = digits ? Number(digits) : Number.NaN;
  displayValue.value = formatValue(model.value);
  element.value = displayValue.value;
  syncValidity();
}

watch(
  () => model.value,
  async (value) => {
    displayValue.value = formatValue(value);
    await nextTick();
    syncValidity();
  },
);
watch(() => props.min, syncValidity);
onMounted(syncValidity);
</script>

<template>
  <input
    ref="input"
    :value="displayValue"
    type="text"
    inputmode="numeric"
    autocomplete="off"
    @input="handleInput"
  />
</template>
