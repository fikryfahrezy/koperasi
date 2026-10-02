import { createRouter, createWebHistory } from "vue-router";
import CashLedger from "../views/CashLedger.vue";
import Loans from "../views/Loans.vue";
import Members from "../views/Members.vue";
import MemberDetail from "../views/MemberDetail.vue";
import Savings from "../views/Savings.vue";
import NewTab from "../views/NewTab.vue";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", redirect: "/new-tab" },
    {
      path: "/new-tab",
      component: NewTab,
      meta: { titleKey: "workspaceTabs.newTab" },
    },
    {
      path: "/members",
      component: Members,
      meta: { titleKey: "sidebar.nav.members" },
    },
    {
      path: "/members/:id",
      name: "member-detail",
      component: MemberDetail,
      meta: { titleKey: "sidebar.nav.members" },
    },
    {
      path: "/savings",
      component: Savings,
      meta: { titleKey: "sidebar.nav.savings", fitViewport: true },
    },
    {
      path: "/loans",
      component: Loans,
      meta: { titleKey: "sidebar.nav.loans", fitViewport: true },
    },
    {
      path: "/cash-ledger",
      component: CashLedger,
      meta: { titleKey: "sidebar.nav.cashLedger" },
    },
    { path: "/:pathMatch(.*)*", redirect: "/new-tab" },
  ],
});
