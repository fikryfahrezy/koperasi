<script setup lang="ts">
import { RouterLink, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import logo from "../assets/logo.png";
import {
  BookOpen,
  ChevronLeft,
  CircleDollarSign,
  Coins,
  FileSpreadsheet,
  Gauge,
  Landmark,
  Settings2,
  ShieldCheck,
  Users,
  X,
} from "lucide-vue-next";
import { navigateActiveWorkspaceTab } from "../workspace-tabs";

defineProps<{ collapsed?: boolean }>();
const emit = defineEmits<{ close: []; toggle: [] }>();
const router = useRouter();

function navigate(path: string) {
  navigateActiveWorkspaceTab(path, router);
  emit("close");
}
const { t } = useI18n();
const mainNav = [
  { to: "/dashboard", labelKey: "sidebar.nav.dashboard", icon: Gauge },
  {
    to: "/transactions",
    labelKey: "sidebar.nav.transactions",
    icon: CircleDollarSign,
  },
  { to: "/members", labelKey: "sidebar.nav.members", icon: Users },
  { to: "/savings", labelKey: "sidebar.nav.savings", icon: Coins },
  { to: "/loans", labelKey: "sidebar.nav.loans", icon: Landmark },
  { to: "/cash-ledger", labelKey: "sidebar.nav.cashLedger", icon: BookOpen },
];
const controlNav = [
  {
    to: "/reconciliation",
    labelKey: "sidebar.nav.reconciliation",
    icon: ShieldCheck,
    badge: "6",
  },
  {
    to: "/migration",
    labelKey: "sidebar.nav.migration",
    icon: FileSpreadsheet,
  },
  { to: "/admin", labelKey: "sidebar.nav.admin", icon: Settings2 },
];
</script>
<template>
  <aside :class="['sidebar', { 'sidebar--collapsed': collapsed }]">
    <header class="sidebar__brand">
      <div class="sidebar__logo">
        <img :src="logo" :alt="t('sidebar.logoAlt')" />
      </div>
      <div class="sidebar__brand-copy">
        <strong>{{ t("sidebar.cooperativeName") }}</strong>
        <span>{{ t("sidebar.cooperativeShortName") }}</span>
      </div>
      <button class="sidebar__mobile-close" @click="emit('close')">
        <X :size="20" />
      </button>
    </header>
    <nav class="sidebar__nav">
      <p>{{ t("sidebar.operational") }}</p>
      <RouterLink
        v-for="item in mainNav"
        :key="item.to"
        :to="item.to"
        class="sidebar__link"
        active-class="sidebar__link--active"
        :aria-label="collapsed ? t(item.labelKey) : undefined"
        :title="collapsed ? t(item.labelKey) : undefined"
        @click="navigate(item.to)"
        ><component :is="item.icon" :size="19" /><span>{{
          t(item.labelKey)
        }}</span></RouterLink
      >
      <p>{{ t("sidebar.controlSystem") }}</p>
      <RouterLink
        v-for="item in controlNav"
        :key="item.to"
        :to="item.to"
        class="sidebar__link"
        active-class="sidebar__link--active"
        :aria-label="collapsed ? t(item.labelKey) : undefined"
        :title="collapsed ? t(item.labelKey) : undefined"
        @click="navigate(item.to)"
        ><component :is="item.icon" :size="19" /><span>{{
          t(item.labelKey)
        }}</span
        ><b v-if="item.badge">{{ item.badge }}</b></RouterLink
      >
    </nav>
    <footer class="sidebar__footer">
      <button
        class="sidebar__collapse"
        :aria-label="collapsed ? t('sidebar.expand') : t('sidebar.collapse')"
        :title="collapsed ? t('sidebar.expand') : t('sidebar.collapse')"
        @click="emit('toggle')"
      >
        <ChevronLeft :size="18" />
      </button>
    </footer>
  </aside>
</template>
