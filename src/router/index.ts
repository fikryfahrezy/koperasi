import { createRouter, createWebHistory } from "vue-router";
import Admin from "../views/Admin.vue";
import Transactions from "../views/Transactions.vue";
import CashLedger from "../views/CashLedger.vue";
import Dashboard from "../views/Dashboard.vue";
import Loans from "../views/Loans.vue";
import Members from "../views/Members.vue";
import Migration from "../views/Migration.vue";
import Reconciliation from "../views/Reconciliation.vue";
import Savings from "../views/Savings.vue";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", redirect: "/dashboard" },
    { path: "/dashboard", component: Dashboard },
    { path: "/transactions", component: Transactions },
    { path: "/members", component: Members },
    { path: "/savings", component: Savings },
    { path: "/loans", component: Loans },
    { path: "/cash-ledger", component: CashLedger },
    { path: "/reconciliation", component: Reconciliation },
    { path: "/migration", component: Migration },
    { path: "/admin", component: Admin },
    { path: "/:pathMatch(.*)*", redirect: "/dashboard" },
  ],
});
