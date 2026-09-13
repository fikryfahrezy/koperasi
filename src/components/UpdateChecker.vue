<script setup lang="ts">
import { onMounted, ref } from "vue";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";
import { DownloadCloud, RefreshCw } from "lucide-vue-next";
import UiModal from "./UiModal.vue";

type Phase =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "installed"
  | "error"
  | "up-to-date";

const open = ref(false);
const phase = ref<Phase>("idle");
const currentVersion = ref("");
const update = ref<Update | null>(null);
const progress = ref(0);
const errorMessage = ref("");

onMounted(async () => {
  currentVersion.value = await getVersion();
  checkForUpdate(true);
});

async function checkForUpdate(silent = false) {
  phase.value = "checking";
  errorMessage.value = "";
  try {
    const result = await check();
    if (result) {
      update.value = result;
      phase.value = "available";
      open.value = true;
    } else {
      phase.value = "up-to-date";
      update.value = null;
      if (!silent) open.value = true;
    }
  } catch (error) {
    phase.value = "error";
    errorMessage.value = error instanceof Error ? error.message : String(error);
    if (!silent) open.value = true;
  }
}

async function installUpdate() {
  if (!update.value) return;
  phase.value = "downloading";
  progress.value = 0;
  let total = 0;
  let downloaded = 0;
  try {
    await update.value.downloadAndInstall((event) => {
      switch (event.event) {
        case "Started":
          total = event.data.contentLength ?? 0;
          break;
        case "Progress":
          downloaded += event.data.chunkLength;
          progress.value =
            total > 0
              ? Math.min(100, Math.round((downloaded / total) * 100))
              : 0;
          break;
        case "Finished":
          progress.value = 100;
          break;
      }
    });
    phase.value = "installed";
    await relaunch();
  } catch (error) {
    phase.value = "error";
    errorMessage.value = error instanceof Error ? error.message : String(error);
  }
}

function closeModal() {
  if (phase.value === "downloading") return;
  open.value = false;
}
</script>

<template>
  <button
    class="icon-button update-checker-button"
    :aria-label="phase === 'available' ? 'Pembaruan tersedia' : 'Cek pembaruan'"
    @click="open = true"
  >
    <RefreshCw :size="19" :class="{ 'is-spinning': phase === 'checking' }" />
    <i v-if="phase === 'available'"></i>
  </button>

  <UiModal
    :open="open"
    title="Pembaruan aplikasi"
    :description="`Versi terpasang saat ini: ${currentVersion}`"
    @close="closeModal"
  >
    <div class="update-checker">
      <div v-if="phase === 'checking'" class="update-checker__state">
        <span class="backend-state__spinner"></span>
        <p>Memeriksa pembaruan…</p>
      </div>

      <div v-else-if="phase === 'available'" class="update-checker__state">
        <DownloadCloud :size="28" />
        <h3>Versi {{ update?.version }} tersedia</h3>
        <p v-if="update?.body" class="update-checker__notes">
          {{ update.body }}
        </p>
        <div class="modal-actions">
          <button class="button button--secondary" @click="open = false">
            Nanti
          </button>
          <button class="button button--primary" @click="installUpdate">
            Update sekarang
          </button>
        </div>
      </div>

      <div v-else-if="phase === 'downloading'" class="update-checker__state">
        <p>Mengunduh dan memasang pembaruan… {{ progress }}%</p>
        <div class="update-checker__progress">
          <div
            class="update-checker__progress-bar"
            :style="{ width: `${progress}%` }"
          ></div>
        </div>
      </div>

      <div v-else-if="phase === 'installed'" class="update-checker__state">
        <p>Pembaruan terpasang. Memulai ulang aplikasi…</p>
      </div>

      <div v-else-if="phase === 'up-to-date'" class="update-checker__state">
        <p>Aplikasi sudah menggunakan versi terbaru.</p>
        <div class="modal-actions">
          <button class="button button--primary" @click="open = false">
            Tutup
          </button>
        </div>
      </div>

      <div v-else-if="phase === 'error'" class="update-checker__state">
        <p class="update-checker__error">
          Gagal memeriksa pembaruan: {{ errorMessage }}
        </p>
        <div class="modal-actions">
          <button
            class="button button--secondary"
            @click="checkForUpdate(false)"
          >
            Coba lagi
          </button>
        </div>
      </div>

      <div v-else class="update-checker__state">
        <div class="modal-actions">
          <button class="button button--primary" @click="checkForUpdate(false)">
            Cek pembaruan
          </button>
        </div>
      </div>
    </div>
  </UiModal>
</template>
