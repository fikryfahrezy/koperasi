<script setup lang="ts">
import { RouterLink } from "vue-router";
import { Landmark, X } from "lucide-vue-next";

const emit = defineEmits(["close"]);
</script>

<template>
  <aside class="sidebar">
    <button
      type="button"
      @click="emit('close')"
      class="sidebar__close"
      aria-label="Close navigation"
    >
      <X class="sidebar__close-icon" />
    </button>

    <div class="sidebar__content">
      <div class="sidebar__brand">
        <p class="sidebar__brand-kicker">Koperasi</p>
        <h1 class="sidebar__brand-title">KOPKAR</h1>
      </div>

      <nav class="sidebar__nav">
        <RouterLink
          to="/loans"
          @click="emit('close')"
          class="sidebar__link"
          active-class="sidebar__link--active"
        >
          <Landmark class="sidebar__link-icon" />
          <span>Pinjaman</span>
        </RouterLink>
      </nav>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 28px 20px 24px 24px;
  border: 1px solid rgba(255, 255, 255, 0.78);
  border-radius: 32px;
  background:
    linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.96),
      rgba(243, 247, 252, 0.92)
    ),
    linear-gradient(160deg, rgba(37, 99, 235, 0.08), transparent 38%);
  box-shadow: var(--shadow-card);
}

.sidebar__close {
  align-self: flex-end;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  margin-bottom: 16px;
  border: 1px solid var(--color-border);
  border-radius: 16px;
  background: rgba(255, 255, 255, 0.88);
  color: var(--color-text-soft);
  cursor: pointer;
  transition:
    border-color 160ms ease,
    color 160ms ease,
    transform 160ms ease;
}

.sidebar__close:hover,
.sidebar__close:focus-visible {
  border-color: var(--color-primary);
  color: var(--color-primary);
  transform: translateY(-1px);
  outline: none;
}

.sidebar__close-icon {
  width: 24px;
  height: 24px;
}

.sidebar__content {
  display: flex;
  flex: 1;
  flex-direction: column;
}

.sidebar__brand {
  padding: 8px 4px 24px;
  border-bottom: 1px solid rgba(185, 200, 214, 0.5);
}

.sidebar__brand-kicker {
  margin: 0 0 8px;
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.26em;
  text-transform: uppercase;
  color: var(--color-primary);
}

.sidebar__brand-title {
  margin: 0;
  font-size: 2rem;
  font-weight: 800;
  letter-spacing: -0.04em;
  color: var(--color-primary-strong);
}

.sidebar__nav {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 28px;
}

.sidebar__link {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 18px 20px;
  border: 1px solid transparent;
  border-radius: 20px;
  background: rgba(241, 245, 249, 0.88);
  color: var(--color-text-muted);
  font-size: 1.05rem;
  font-weight: 700;
  transition:
    background-color 160ms ease,
    border-color 160ms ease,
    color 160ms ease,
    transform 160ms ease,
    box-shadow 160ms ease;
}

.sidebar__link:hover,
.sidebar__link:focus-visible {
  border-color: rgba(29, 78, 216, 0.16);
  background: rgba(219, 234, 254, 0.72);
  color: var(--color-primary-strong);
  transform: translateY(-1px);
  outline: none;
}

.sidebar__link--active {
  border-color: rgba(29, 78, 216, 0.18);
  background: linear-gradient(
    135deg,
    rgba(219, 234, 254, 0.96),
    rgba(239, 246, 255, 0.92)
  );
  color: var(--color-primary-strong);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.76);
}

.sidebar__link-icon {
  width: 24px;
  height: 24px;
  flex-shrink: 0;
}

@media (min-width: 1200px) {
  .sidebar__close {
    display: none;
  }
}
</style>
