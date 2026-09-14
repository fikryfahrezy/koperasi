<script setup lang="ts">
import { RouterLink, useRouter } from "vue-router";
import {
  BookOpen,
  CircleDollarSign,
  Coins,
  FileSpreadsheet,
  Gauge,
  Landmark,
  Settings2,
  ShieldCheck,
  Users,
} from "lucide-vue-next";
import { useI18n } from "vue-i18n";
import { navigateActiveWorkspaceTab } from "../workspace-tabs";

const router = useRouter();
const { t } = useI18n();
const menuItems = [
  {
    to: "/dashboard",
    labelKey: "sidebar.nav.dashboard",
    icon: Gauge,
    hidden: true,
  },
  {
    to: "/transactions",
    labelKey: "sidebar.nav.transactions",
    icon: CircleDollarSign,
    hidden: true,
  },
  { to: "/cash-ledger", labelKey: "sidebar.nav.cashLedger", icon: BookOpen },
  { to: "/savings", labelKey: "sidebar.nav.savings", icon: Coins },
  { to: "/loans", labelKey: "sidebar.nav.loans", icon: Landmark },
  { to: "/members", labelKey: "sidebar.nav.members", icon: Users },
  {
    to: "/reconciliation",
    labelKey: "sidebar.nav.reconciliation",
    icon: ShieldCheck,
    badge: "6",
    hidden: true,
  },
  {
    to: "/migration",
    labelKey: "sidebar.nav.migration",
    icon: FileSpreadsheet,
    divider: true,
  },
  {
    to: "/admin",
    labelKey: "sidebar.nav.admin",
    icon: Settings2,
    hidden: true,
  },
];
const visibleMenuItems = menuItems.filter((item) => !item.hidden);

function navigate(path: string) {
  navigateActiveWorkspaceTab(path, router);
}
</script>

<template>
  <nav class="app-menu" :aria-label="t('app.mainMenu')">
    <div class="app-menu__scroll">
      <RouterLink
        v-for="item in visibleMenuItems"
        :key="item.to"
        :to="item.to"
        :class="['app-menu__item', { 'app-menu__item--divider': item.divider }]"
        active-class="app-menu__item--active"
        @click="navigate(item.to)"
      >
        <component :is="item.icon" :size="16" />
        <span>{{ t(item.labelKey) }}</span>
        <b v-if="item.badge">{{ item.badge }}</b>
      </RouterLink>
    </div>
  </nav>
</template>
