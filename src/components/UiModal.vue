<script setup lang="ts">
import { X } from "lucide-vue-next";

defineProps<{
  open: boolean;
  title: string;
  description?: string;
  size?: "md" | "lg";
}>();
const emit = defineEmits<{ close: [] }>();
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="open" class="modal-backdrop" @click.self="emit('close')">
        <section
          :class="['modal-card', { 'modal-card--lg': size === 'lg' }]"
          role="dialog"
          aria-modal="true"
          :aria-label="title"
        >
          <header class="modal-card__header">
            <div>
              <h2>{{ title }}</h2>
              <p v-if="description">{{ description }}</p>
            </div>
            <button
              class="icon-button"
              type="button"
              aria-label="Tutup"
              @click="emit('close')"
            >
              <X :size="20" />
            </button>
          </header>
          <div class="modal-card__body"><slot /></div>
        </section>
      </div>
    </Transition>
  </Teleport>
</template>
