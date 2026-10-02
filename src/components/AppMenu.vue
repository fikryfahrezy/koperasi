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
