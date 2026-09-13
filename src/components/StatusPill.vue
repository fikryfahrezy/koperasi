<script setup lang="ts">
defineProps<{
  label: string;
  tone?: "success" | "warning" | "danger" | "info" | "neutral";
}>();
</script>

<template>
  <span :class="['status-pill', `status-pill--${tone ?? 'neutral'}`]">
    <span class="status-pill__dot"></span>{{ label }}
  </span>
</template>
