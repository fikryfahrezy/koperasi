<script setup lang="ts">
import { computed } from "vue";
import { ChevronDown } from "lucide-vue-next";

export type SelectValue = string | number;

export interface SelectOption {
  value: SelectValue;
  label: string;
  disabled?: boolean;
}

const props = withDefaults(
  defineProps<{
    modelValue: SelectValue;
    options: SelectOption[];
    ariaLabel: string;
    label?: string;
    placeholder?: string;
    disabled?: boolean;
    required?: boolean;
    variant?: "field" | "toolbar";
  }>(),
  {
    placeholder: undefined,
    label: undefined,
    disabled: false,
    required: false,
    variant: "field",
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: SelectValue];
}>();

const selectedLabel = computed(
  () =>
    props.options.find((option) => option.value === props.modelValue)?.label ??
    props.placeholder ??
    "",
);

function updateValue(event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  const option = props.options.find((item) => String(item.value) === value);
  emit("update:modelValue", option?.value ?? value);
}
</script>

<template>
  <div
    class="ui-select"
    :class="[`ui-select--${variant}`, { 'ui-select--disabled': disabled }]"
  >
    <slot name="icon" />
    <span v-if="label" class="ui-select__label">{{ label }}:</span>
    <span class="ui-select__value">{{ selectedLabel }}</span>
    <ChevronDown class="ui-select__chevron" :size="14" />
    <select
      :value="modelValue"
      :aria-label="ariaLabel"
      :disabled="disabled"
      :required="required"
      @change="updateValue"
    >
      <option v-if="placeholder" value="" disabled>{{ placeholder }}</option>
      <option
        v-for="option in options"
        :key="option.value"
        :value="option.value"
        :disabled="option.disabled"
      >
        {{ option.label }}
      </option>
    </select>
  </div>
</template>

<style scoped>
.ui-select {
  position: relative;
  cursor: pointer;
}
.ui-select--toolbar {
  min-height: 34px;
  padding: 0 12px;
}
.ui-select--field {
  width: 100%;
  min-height: 43px;
  padding: 0 11px;
  color: var(--ink);
  font-weight: 400;
}
.ui-select--field:focus-within {
  border-color: #91acbd;
}
.ui-select--disabled {
  color: var(--soft);
  background: #f3f5f4;
  cursor: default;
}
.ui-select__label {
  font-weight: 400;
}
.ui-select__value {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ui-select__chevron,
.ui-select > svg {
  flex: none;
  pointer-events: none;
}
.ui-select select {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  padding: 0;
  border: 0;
  outline: 0;
  opacity: 0;
  cursor: pointer;
  appearance: none;
}
</style>
