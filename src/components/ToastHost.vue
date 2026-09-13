<script setup lang="ts">
import { CheckCircle2, Info, TriangleAlert } from "lucide-vue-next";
import { useKoperasiStore } from "../store/koperasi";

const { toasts } = useKoperasiStore();
</script>

<template>
  <div class="toast-stack" aria-live="polite">
    <TransitionGroup name="toast">
      <article
        v-for="toast in toasts"
        :key="toast.id"
        :class="['toast-card', `toast-card--${toast.tone}`]"
      >
        <CheckCircle2 v-if="toast.tone === 'success'" :size="20" />
        <TriangleAlert v-else-if="toast.tone === 'warning'" :size="20" />
        <Info v-else :size="20" />
        <div>
          <strong>{{ toast.title }}</strong>
          <p>{{ toast.message }}</p>
        </div>
      </article>
    </TransitionGroup>
  </div>
</template>
