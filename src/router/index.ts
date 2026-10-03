import { createRouter, createWebHistory } from "vue-router";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", redirect: "/new-tab" },
    {
      path: "/new-tab",
      component: () => import("../views/NewTab.vue"),
      meta: { titleKey: "workspaceTabs.newTab" },
    },
    {
      path: "/members",
      component: () => import("../views/Members.vue"),
      meta: { titleKey: "sidebar.nav.members", fitViewport: true },
    },
    {
      path: "/members/:id",
      name: "member-detail",
      component: () => import("../views/MemberDetail.vue"),
      meta: { titleKey: "sidebar.nav.members" },
    },
    {
      path: "/savings",
      component: () => import("../views/Savings.vue"),
      meta: { titleKey: "sidebar.nav.savings", fitViewport: true },
    },
    {
      path: "/loans",
      component: () => import("../views/Loans.vue"),
      meta: { titleKey: "sidebar.nav.loans", fitViewport: true },
    },
    {
      path: "/cash-ledger",
      component: () => import("../views/CashLedger.vue"),
      meta: { titleKey: "sidebar.nav.cashLedger", fitViewport: true },
    },
    { path: "/:pathMatch(.*)*", redirect: "/new-tab" },
  ],
});
