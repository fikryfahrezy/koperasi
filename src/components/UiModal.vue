<script setup lang="ts">
import { onActivated, onDeactivated, ref } from "vue";
import { X } from "lucide-vue-next";
import { useI18n } from "vue-i18n";

defineProps<{
  open: boolean;
  title: string;
  description?: string;
  size?: "md" | "lg";
  /** App dialogs cover the whole window; page dialogs leave tabs usable. */
  scope?: "page" | "app";
}>();
const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();
// Cached pages stay mounted while another tab shows, but teleported content
// would stay on screen, so hide the dialog until its page is shown again.
const pageVisible = ref(true);
onActivated(() => (pageVisible.value = true));
onDeactivated(() => (pageVisible.value = false));
</script>

<template>
  <Teleport :to="scope === 'app' ? 'body' : '#page-overlay'" defer>
    <div
      v-if="open"
      v-show="pageVisible"
      :class="['modal-backdrop', { 'modal-backdrop--app': scope === 'app' }]"
      @click.self="emit('close')"
    >
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
            :aria-label="t('modal.close')"
            @click="emit('close')"
          >
            <X :size="20" />
          </button>
        </header>
        <div class="modal-card__body"><slot /></div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-backdrop {
  position: absolute;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: rgba(11, 21, 31, 0.48);
  backdrop-filter: blur(5px);
}
.modal-backdrop--app {
  position: fixed;
}
.modal-card {
  width: min(100%, 520px);
  max-height: 100%;
  overflow-y: auto;
  border-radius: 17px;
  background: #fff;
}
.modal-card--lg {
  width: min(100%, 720px);
}
.modal-card__header {
  display: flex;
  justify-content: space-between;
  gap: 20px;
  padding: 22px 24px 17px;
  border-bottom: 1px solid var(--line);
}
.modal-card__header h2 {
  margin: 0 0 5px;
  font-size: 18px;
}
.modal-card__header p {
  margin: 0;
  color: var(--muted);
  font-size: 11px;
}
.modal-card__body {
  padding: 22px 24px 24px;
}
</style>
