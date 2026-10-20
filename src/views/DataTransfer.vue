<script setup lang="ts">
import { onMounted, ref } from "vue";
import {
  DatabaseBackup,
  FolderOpen,
  TriangleAlert,
  Upload,
} from "lucide-vue-next";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import PageHeader from "../components/PageHeader.vue";
import UiModal from "../components/UiModal.vue";
import { useKoperasiStore } from "../store/koperasi";

// Moves the whole database between computers without opening %APPDATA% or
// Application Support by hand.
const { databaseLocation, exportDatabase, importDatabase, notify } =
  useKoperasiStore();

const location = ref("");
const exporting = ref(false);
const importing = ref(false);
const confirmImport = ref(false);

onMounted(async () => {
  try {
    location.value = await databaseLocation();
  } catch {
    location.value = "";
  }
});

async function handleExport() {
  if (exporting.value) return;
  exporting.value = true;
  try {
    await exportDatabase();
  } finally {
    exporting.value = false;
  }
}

async function handleImport() {
  if (importing.value) return;
  confirmImport.value = false;
  importing.value = true;
  try {
    // Only returns when cancelled or refused; otherwise the app restarts.
    await importDatabase();
  } finally {
    importing.value = false;
  }
}

async function openFolder() {
  try {
    await revealItemInDir(location.value);
  } catch (error) {
    notify(
      "Folder gagal dibuka",
      error instanceof Error ? error.message : String(error),
      "warning",
    );
  }
}
</script>

<template>
  <div class="page-stack data-page">
    <PageHeader title="Database" />

    <section class="panel data-card">
      <span class="data-card__icon"><DatabaseBackup :size="20" /></span>
      <div class="data-card__body">
        <h2>Ekspor database</h2>
        <p>
          Simpan salinan seluruh data (anggota, simpanan, pinjaman, buku kas,
          dan histori transaksi) ke satu file <code>.db</code>. Pakai file ini
          sebagai cadangan atau untuk memindahkan data ke komputer lain.
        </p>
      </div>
      <button
        class="button button--primary"
        type="button"
        :disabled="exporting"
        @click="handleExport"
      >
        <DatabaseBackup :size="16" />
        {{ exporting ? "Mengekspor…" : "Ekspor database" }}
      </button>
    </section>

    <section class="panel data-card">
      <span class="data-card__icon data-card__icon--amber"
        ><Upload :size="20"
      /></span>
      <div class="data-card__body">
        <h2>Impor database</h2>
        <p>
          Ganti seluruh data di aplikasi ini dengan file hasil ekspor. Data saat
          ini disalin dulu ke folder data, lalu aplikasi dimulai ulang.
        </p>
      </div>
      <button
        class="button button--secondary"
        type="button"
        :disabled="importing"
        @click="confirmImport = true"
      >
        <Upload :size="16" />
        {{ importing ? "Mengimpor…" : "Impor database" }}
      </button>
    </section>

    <section v-if="location" class="panel data-location">
      <div>
        <span>Lokasi database saat ini</span>
        <code>{{ location }}</code>
      </div>
      <button
        class="button button--secondary"
        type="button"
        @click="openFolder"
      >
        <FolderOpen :size="16" />
        Buka folder
      </button>
    </section>

    <UiModal
      :open="confirmImport"
      title="Impor database?"
      @close="confirmImport = false"
    >
      <div class="form-stack">
        <p class="data-warning">
          <TriangleAlert :size="18" />
          <span>
            Semua data di aplikasi ini akan diganti dengan isi file yang
            dipilih. Salinan data saat ini disimpan sebagai
            <code>koperasi-sebelum-impor-….db</code> di folder data, lalu
            aplikasi dimulai ulang.
          </span>
        </p>
        <div class="modal-actions">
          <button
            class="button button--secondary"
            type="button"
            @click="confirmImport = false"
          >
            Batal
          </button>
          <button
            class="button button--primary"
            type="button"
            @click="handleImport"
          >
            Pilih file dan impor
          </button>
        </div>
      </div>
    </UiModal>
  </div>
</template>

<style scoped>
.data-page {
  max-width: 820px;
  gap: 14px;
}
.data-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 18px 20px;
}
.data-card__icon {
  flex: 0 0 auto;
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  color: var(--green);
  background: var(--green-soft);
}
.data-card__icon--amber {
  color: var(--amber);
  background: var(--amber-soft);
}
.data-card__body {
  flex: 1 1 auto;
  min-width: 0;
}
.data-card h2 {
  margin: 0 0 4px;
  font-size: 14px;
}
.data-card p {
  margin: 0;
  color: var(--muted);
  font-size: 12px;
  line-height: 1.5;
}
.data-card .button {
  flex: 0 0 auto;
}
.data-location {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 14px 20px;
}
.data-location > div {
  flex: 1 1 auto;
  min-width: 0;
  display: grid;
  gap: 4px;
}
.data-location span {
  color: var(--muted);
  font-size: 11px;
}
.data-location code {
  overflow-wrap: anywhere;
  font-size: 11.5px;
}
.data-warning {
  display: flex;
  gap: 10px;
  margin: 0;
  padding: 12px 14px;
  color: var(--amber);
  background: var(--amber-soft);
  font-size: 12px;
  line-height: 1.5;
}
.data-warning svg {
  flex: 0 0 auto;
}
@media (max-width: 640px) {
  .data-card,
  .data-location {
    flex-wrap: wrap;
  }
}
</style>
