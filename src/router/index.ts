import { createRouter, createWebHistory } from "vue-router";
import Admin from "../views/Admin.vue";
import Transactions from "../views/Transactions.vue";
import CashLedger from "../views/CashLedger.vue";
import Dashboard from "../views/Dashboard.vue";
import Loans from "../views/Loans.vue";
import Members from "../views/Members.vue";
import MemberDetail from "../views/MemberDetail.vue";
import Migration from "../views/Migration.vue";
import Reconciliation from "../views/Reconciliation.vue";
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
      path: "/dashboard",
      component: Dashboard,
      meta: { titleKey: "sidebar.nav.dashboard" },
    },
    {
      path: "/transactions",
      component: Transactions,
      meta: { titleKey: "sidebar.nav.transactions" },
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
      meta: { titleKey: "sidebar.nav.savings" },
    },
    {
      path: "/loans",
      component: Loans,
      meta: { titleKey: "sidebar.nav.loans" },
    },
    {
      path: "/cash-ledger",
      component: CashLedger,
      meta: { titleKey: "sidebar.nav.cashLedger" },
    },
    {
      path: "/reconciliation",
      component: Reconciliation,
      meta: { titleKey: "sidebar.nav.reconciliation" },
    },
    {
      path: "/migration",
      component: Migration,
      meta: { titleKey: "sidebar.nav.migration" },
    },
    {
      path: "/admin",
      component: Admin,
      meta: { titleKey: "sidebar.nav.admin" },
    },
    { path: "/:pathMatch(.*)*", redirect: "/dashboard" },
  ],
});
