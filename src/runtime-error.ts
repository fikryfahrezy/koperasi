import { ref } from "vue";

export const runtimeError = ref<string | null>(null);

const stringifyError = (value: unknown): string => {
  if (value instanceof Error) {
    return value.stack ?? `${value.name}: ${value.message}`;
  }

  if (typeof value === "string") {
    return value;
  }

  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
};

export const reportRuntimeError = (value: unknown, context?: string) => {
  const details = stringifyError(value);
  runtimeError.value = context ? `${context}\n\n${details}` : details;
};

export const clearRuntimeError = () => {
  runtimeError.value = null;
};