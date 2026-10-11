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

<style scoped>
.status-pill {
  width: max-content;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 8px;
  border-radius: 999px;
  color: #64716d;
  background: #eef1f0;
  font-size: 9.5px;
  font-weight: 700;
  white-space: nowrap;
}
.status-pill__dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: currentColor;
}
.status-pill--success {
  color: var(--green);
  background: var(--green-soft);
}
.status-pill--warning {
  color: var(--amber);
  background: var(--amber-soft);
}
.status-pill--danger {
  color: var(--red);
  background: var(--red-soft);
}
.status-pill--info {
  color: var(--blue);
  background: var(--blue-soft);
}
</style>
