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
    placeholder?: string;
    disabled?: boolean;
    required?: boolean;
    variant?: "field" | "toolbar";
  }>(),
  {
    placeholder: undefined,
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
