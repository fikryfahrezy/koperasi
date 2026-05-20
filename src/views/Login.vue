<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { KeyRound, User } from "lucide-vue-next";

const router = useRouter();
const username = ref("");
const password = ref("");
const isError = ref(false);

function onLogin() {
  if (username.value && password.value) {
    // For mock, any non-empty credential works
    router.push("/loans");
  } else {
    isError.value = true;
  }
}
</script>

<template>
  <div
    class="min-h-screen bg-blue-50 flex flex-col justify-center py-12 sm:px-6 lg:px-8"
  >
    <div class="sm:mx-auto sm:w-full sm:max-w-xl">
      <div class="text-center">
        <h1 class="text-5xl font-extrabold text-blue-800 tracking-tight">
          KOPKAR
        </h1>
        <h2 class="mt-6 text-3xl font-bold text-gray-900">
          Masuk ke Sistem Koperasi
        </h2>
      </div>

      <div
        class="mt-12 bg-white py-12 px-10 shadow-2xl sm:rounded-3xl border border-gray-100"
      >
        <form class="space-y-8" @submit.prevent="onLogin">
          <div
            v-if="isError"
            class="bg-red-50 text-red-700 p-6 rounded-2xl text-xl font-bold border border-red-200"
          >
            Pastikan nama pengguna dan kata sandi telah diisi dengan benar.
          </div>

          <div>
            <label
              for="username"
              class="block text-2xl font-bold text-gray-700 mb-4"
            >
              Nama Pengguna
            </label>
            <div class="mt-1 relative rounded-md shadow-sm">
              <div
                class="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none"
              >
                <User class="h-8 w-8 text-gray-400" />
              </div>
              <input
                id="username"
                v-model="username"
                type="text"
                class="focus:ring-blue-500 focus:border-blue-500 block w-full pl-16 pr-6 py-6 text-2xl border-gray-300 rounded-2xl bg-gray-50 border outline-none transition-shadow"
                placeholder="Masukkan admin"
              />
            </div>
          </div>

          <div>
            <label
              for="password"
              class="block text-2xl font-bold text-gray-700 mb-4"
            >
              Kata Sandi
            </label>
            <div class="mt-1 relative rounded-md shadow-sm">
              <div
                class="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none"
              >
                <KeyRound class="h-8 w-8 text-gray-400" />
              </div>
              <input
                id="password"
                v-model="password"
                type="password"
                class="focus:ring-blue-500 focus:border-blue-500 block w-full pl-16 pr-6 py-6 text-2xl border-gray-300 rounded-2xl bg-gray-50 border outline-none transition-shadow"
                placeholder="Masukkan kata sandi"
              />
            </div>
          </div>

          <div class="pt-6">
            <button
              type="submit"
              class="w-full flex justify-center py-6 px-8 border border-transparent rounded-2xl shadow-md text-3xl font-extrabold text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-4 focus:ring-offset-2 focus:ring-blue-500 transition-colors"
            >
              Masuk Sistem
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>
