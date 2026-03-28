<script setup lang="ts">
import { computed, ref } from "vue";
import { RouterView, useRoute } from "vue-router";
import Sidebar from "./components/Sidebar.vue";
import { Menu } from "lucide-vue-next";

const route = useRoute();
const isLogin = computed(() => route.path === "/login");
const isSidebarOpen = ref(false);
</script>

<template>
  <div v-if="isLogin">
    <RouterView />
  </div>
  <div v-else class="flex h-screen bg-gray-50 text-gray-900 font-sans relative">
    <!-- Overlay untuk layar kecil saat sidebar terbuka -->
    <div
      v-if="isSidebarOpen"
      class="fixed inset-0 bg-gray-900/50 z-20 xl:hidden"
      @click="isSidebarOpen = false"
    ></div>

    <!-- Sidebar -->
    <div
      :class="[
        'absolute xl:relative z-30 transform transition-transform duration-300 h-full',
        isSidebarOpen ? 'translate-x-0' : '-translate-x-full xl:translate-x-0',
      ]"
    >
      <Sidebar @close="isSidebarOpen = false" />
    </div>

    <!-- Konten Utama -->
    <div class="flex-1 flex flex-col overflow-hidden w-full">
      <header
        class="bg-white shadow px-4 sm:px-6 py-4 sm:py-6 border-b border-gray-200 flex items-center justify-between"
      >
        <div class="flex items-center gap-4">
          <button
            @click="isSidebarOpen = !isSidebarOpen"
            class="xl:hidden p-2 -ml-2 rounded-xl text-gray-600 hover:bg-gray-100 transition focus:outline-none focus:ring-4 focus:ring-blue-100"
          >
            <Menu class="w-8 h-8" />
          </button>
          <h2
            class="text-2xl sm:text-3xl font-extrabold text-[#1f2937] truncate"
          >
            Koperasi Sejahtera
          </h2>
        </div>
        <div class="text-lg sm:text-xl font-medium text-gray-600">Admin</div>
      </header>
      <main
        class="flex-1 overflow-x-hidden overflow-y-auto bg-gray-50 p-4 sm:p-8"
      >
        <RouterView />
      </main>
    </div>
  </div>
</template>
