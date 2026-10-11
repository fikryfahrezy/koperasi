<script setup lang="ts">
import { RouterLink, useRouter } from "vue-router";
import { BookOpen, Coins, HandCoins, Users } from "lucide-vue-next";
import { useI18n } from "vue-i18n";
import { navigateActiveWorkspaceTab } from "../workspace-tabs";

const router = useRouter();
const { t } = useI18n();
const menuItems = [
  { to: "/cash-ledger", labelKey: "sidebar.nav.cashLedger", icon: BookOpen },
  { to: "/savings", labelKey: "sidebar.nav.savings", icon: Coins },
  { to: "/loans", labelKey: "sidebar.nav.loans", icon: HandCoins },
  { to: "/members", labelKey: "sidebar.nav.members", icon: Users },
];

function navigate(path: string) {
  navigateActiveWorkspaceTab(path, router);
}
</script>

<template>
  <nav class="app-menu" :aria-label="t('app.mainMenu')">
    <div class="app-menu__scroll">
      <RouterLink
        v-for="item in menuItems"
        :key="item.to"
        :to="item.to"
        class="app-menu__item"
        active-class="app-menu__item--active"
        @click="navigate(item.to)"
      >
        <component :is="item.icon" :size="16" />
        <span>{{ t(item.labelKey) }}</span>
      </RouterLink>
    </div>
  </nav>
</template>

<style scoped>
.app-menu {
  flex-shrink: 0;
  z-index: 18;
  height: 45px;
  border-top: 1px solid var(--line-strong);
  background: #f3f6f4;
  overflow: hidden;
}
.app-menu__scroll {
  height: 100%;
  display: flex;
  align-items: stretch;
  gap: 2px;
  padding: 0 var(--shell-gutter);
  overflow-x: auto;
  scrollbar-width: none;
}
.app-menu__scroll::-webkit-scrollbar {
  display: none;
}
.app-menu__item {
  position: relative;
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 0 11px;
  color: var(--muted);
  font-size: 11.5px;
  font-weight: 600;
  white-space: nowrap;
}
.app-menu__item:hover {
  color: var(--green);
  background: var(--surface-soft);
}
.app-menu__item--active {
  color: var(--green);
  background: #fff;
}
.app-menu__item--active::after {
  content: "";
  position: absolute;
  inset: 0 8px auto;
  height: 3px;
  background: var(--green);
}
.app-menu__item b {
  min-width: 17px;
  padding: 2px 5px;
  color: var(--amber);
  background: var(--amber-soft);
  font-size: 9px;
  text-align: center;
}
@media (max-width: 1100px) {
  .app-menu__scroll {
    padding-inline: 14px;
  }
}
@media (max-width: 780px) {
  .app-menu {
    height: 43px;
  }
  .app-menu__scroll {
    padding-inline: var(--shell-gutter);
  }
  .app-menu__item {
    padding-inline: 10px;
  }
}
</style>
