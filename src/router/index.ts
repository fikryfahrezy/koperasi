import { createRouter, createWebHistory } from "vue-router";
import Loans from "../views/Loans.vue";
import Login from "../views/Login.vue";

const routes = [
  {
    path: "/",
    redirect: "/login",
  },
  {
    path: "/login",
    name: "Login",
    component: Login,
  },
  {
    path: "/loans",
    name: "Loans",
    component: Loans,
  },
];

export const router = createRouter({
  history: createWebHistory(),
  routes,
});
