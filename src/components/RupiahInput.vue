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

function countDigits(text: string) {
  return text.replace(/\D/g, "").length;
}

/** Position just after the `count`-th digit of `text`, or its end. */
function caretAfterDigits(text: string, count: number) {
  if (count <= 0) {
    const first = text.search(/\d/);
    return first === -1 ? text.length : first;
  }
  let seen = 0;
  for (let index = 0; index < text.length; index++) {
    if (/\d/.test(text[index]) && ++seen === count) return index + 1;
  }
  return text.length;
}

function handleInput(event: Event) {
  const element = event.target as HTMLInputElement;
  const inputType = (event as InputEvent).inputType ?? "";
  let raw = element.value;
  let caret = element.selectionStart ?? raw.length;

  // Deleting only a thousands separator removes the digit next to it, so the
  // keystroke is not lost when the separator is put back.
  if (countDigits(raw) === countDigits(displayValue.value)) {
    if (inputType === "deleteContentBackward") {
      const before = raw.slice(0, caret).search(/\d\D*$/);
      if (before !== -1) {
        raw = raw.slice(0, before) + raw.slice(before + 1);
        caret = before;
      }
    } else if (inputType === "deleteContentForward") {
      const offset = raw.slice(caret).search(/\d/);
      if (offset !== -1) {
        raw = raw.slice(0, caret + offset) + raw.slice(caret + offset + 1);
      }
    }
  }

  const digits = raw.replace(/\D/g, "");
  // Leading zeros disappear once the number is formatted.
  const leadingZeros = digits.length - digits.replace(/^0+/, "").length;
  const digitsBeforeCaret = Math.max(
    0,
    countDigits(raw.slice(0, caret)) - leadingZeros,
  );

  // Format the new number itself: with a parent v-model, `model.value` still
  // holds the old one until the parent re-renders, which rewrites the input
  // and moves the caret to the end.
  const value = digits ? Number(digits) : Number.NaN;
  model.value = value;
  displayValue.value = formatValue(value);
  element.value = displayValue.value;
  if (document.activeElement === element) {
    const position = caretAfterDigits(displayValue.value, digitsBeforeCaret);
    element.setSelectionRange(position, position);
  }
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
