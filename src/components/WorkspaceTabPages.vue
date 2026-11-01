<script setup lang="ts">
import {
  computed,
  nextTick,
  onMounted,
  provide,
  reactive,
  ref,
  watch,
} from "vue";
import {
  RouterView,
  routeLocationKey,
  routerViewLocationKey,
  useRouter,
} from "vue-router";
import CachedWorkspacePage from "./CachedWorkspacePage";
import {
  activeWorkspaceTab,
  activeWorkspaceTabId,
  workspaceTabs,
} from "../workspace-tabs";
import NewTab from "../views/NewTab.vue";
import Dashboard from "../views/Dashboard.vue";
import BalanceSheet from "../views/BalanceSheet.vue";
import Members from "../views/Members.vue";
import Savings from "../views/Savings.vue";
import Loans from "../views/Loans.vue";
import CashLedger from "../views/CashLedger.vue";

const router = useRouter();
const tabRoute = computed(() =>
  router.resolve(activeWorkspaceTab()?.fullPath ?? "/new-tab"),
);
// Hidden pages must observe their own tab's route, including member searches.
provide(
  routeLocationKey,
  reactive({
    name: computed(() => tabRoute.value.name),
    path: computed(() => tabRoute.value.path),
    fullPath: computed(() => tabRoute.value.fullPath),
    query: computed(() => tabRoute.value.query),
    hash: computed(() => tabRoute.value.hash),
    params: computed(() => tabRoute.value.params),
    matched: computed(() => tabRoute.value.matched),
    meta: computed(() => tabRoute.value.meta),
    redirectedFrom: computed(() => tabRoute.value.redirectedFrom),
  }),
);
provide(routerViewLocationKey, tabRoute);

const pages = [
  { path: "/new-tab", component: NewTab },
  { path: "/dashboard", component: Dashboard },
  { path: "/members", component: Members },
  { path: "/savings", component: Savings },
  { path: "/loans", component: Loans },
  { path: "/cash-ledger", component: CashLedger },
  { path: "/balance-sheet", component: BalanceSheet },
];
const warming = ref(true);
const isDetail = computed(
  () => !pages.some((page) => page.path === tabRoute.value.path),
);
// Detail pages share one cached instance, so keep their scroll per tab here.
const root = ref<HTMLElement>();
const detailScroll = new Map<string, { path: string; top: number }>();
const content = () => root.value?.closest<HTMLElement>(".app-content");
let revision = 0;
watch(
  () =>
    [isDetail.value, tabRoute.value.path, activeWorkspaceTabId.value] as const,
  async ([detail, path, tabId], [wasDetail, previousPath, previousTabId]) => {
    const currentRevision = ++revision;
    const element = content();
    if (wasDetail && element)
      detailScroll.set(previousTabId, {
        path: previousPath,
        top: element.scrollTop,
      });
    if (!detail) return;
    await nextTick();
    if (revision !== currentRevision) return;
    const saved = detailScroll.get(tabId);
    const target = content();
    if (target) target.scrollTop = saved?.path === path ? saved.top : 0;
  },
  { flush: "pre" },
);
watch(
  () => workspaceTabs.value.map((tab) => tab.id),
  (ids) => {
    for (const id of detailScroll.keys())
      if (!ids.includes(id)) detailScroll.delete(id);
  },
);
onMounted(async () => {
  // Mount every main page once, then retain its rendered DOM in KeepAlive.
  await nextTick();
  warming.value = false;
});
</script>

<template>
  <div ref="root" class="workspace-tab-pages">
    <CachedWorkspacePage
      v-for="page in pages"
      :key="page.path"
      :page="page.component"
      :active="tabRoute.path === page.path"
      :warming="warming"
      :tab-id="activeWorkspaceTabId"
      :open-tab-ids="workspaceTabs.map((tab) => tab.id)"
    />
    <RouterView v-slot="{ Component }">
      <KeepAlive>
        <component v-if="isDetail" :is="Component" />
      </KeepAlive>
    </RouterView>
  </div>
</template>

<style scoped>
.workspace-tab-pages {
  display: flex;
  flex-direction: column;
  flex: 1 1 0%;
  min-height: 0;
}
</style>
