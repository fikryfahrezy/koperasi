<script setup lang="ts">
import { RouterLink } from "vue-router";
import { Landmark, X } from "lucide-vue-next";

const emit = defineEmits(["close"]);
</script>

<template>
  <div
    class="h-screen w-80 bg-white border-r border-gray-200 flex flex-col justify-between shadow-xl xl:shadow-sm flex-shrink-0 relative"
  >
    <!-- Close button for mobile -->
    <button
      @click="emit('close')"
      class="xl:hidden absolute top-6 right-6 p-2 rounded-xl text-gray-500 hover:bg-gray-100 hover:text-gray-800 transition focus:outline-none focus:ring-4 focus:ring-blue-100"
    >
      <X class="w-8 h-8" />
    </button>

    <div>
      <div
        class="h-24 flex items-center justify-center border-b border-gray-100 xl:mt-0 pt-2 xl:pt-0"
      >
        <h1 class="text-3xl font-bold text-blue-600 tracking-tight">KOPKAR</h1>
      </div>
      <nav class="flex-1 mt-6 px-4 space-y-3">
        <RouterLink
          to="/loans"
          @click="emit('close')"
          class="flex items-center space-x-4 px-6 py-5 bg-gray-50 rounded-xl hover:bg-blue-50 text-gray-700 hover:text-blue-700 transition"
          active-class="bg-blue-100 text-blue-800 font-semibold shadow-inner"
        >
          <Landmark class="w-8 h-8" />
          <span class="text-xl">Pinjaman</span>
        </RouterLink>
      </nav>
    </div>
  </div>
</template>
