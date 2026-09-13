<script setup lang="ts">
defineProps<{ eyebrow?: string; title: string; description: string }>();
</script>

<template>
  <header class="page-heading">
    <div>
      <p v-if="eyebrow" class="page-heading__eyebrow">{{ eyebrow }}</p>
      <h1>{{ title }}</h1>
      <p>{{ description }}</p>
    </div>
    <div v-if="$slots.actions" class="page-heading__actions">
      <slot name="actions" />
    </div>
  </header>
</template>
