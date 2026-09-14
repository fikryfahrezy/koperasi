<script setup lang="ts">
import { onMounted, reactive, ref } from "vue";
import { Clock3, Save, Shield } from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import StatusPill from "../components/StatusPill.vue";
import {
  type FinancialParameters,
  formatCurrency,
  useKoperasiStore,
} from "../store/koperasi";

const { admin, loadAdminState, saveFinancialParameters } = useKoperasiStore();
const tab = ref<"parameter" | "audit">("parameter");
const saving = ref(false);
const form = reactive<FinancialParameters>({
  principalSavings: 50_000,
  mandatorySavings: 50_000,
  provisionRate: 1,
  annualRate: 24,
  effectiveDate: "2026-10-01",
});
onMounted(async () => {
  if (!(await loadAdminState())) return;
  Object.assign(form, {
    principalSavings: admin.parameters.principalSavings,
    mandatorySavings: admin.parameters.mandatorySavings,
    provisionRate: admin.parameters.provisionRate,
    annualRate: admin.parameters.annualRate,
  });
});

async function save() {
  saving.value = true;
  await saveFinancialParameters({ ...form });
  saving.value = false;
}

function eventLabel(action: string) {
  const labels: Record<string, string> = {
    CREATED: "Anggota dibuat",
    DRAFT_CREATED: "Draf pinjaman dibuat",
    DISBURSED: "Pinjaman dicairkan",
    POSTED: "Transaksi diposting",
    REVERSED: "Transaksi dibalik",
    VERSION_CREATED: "Versi parameter dibuat",
  };
  return labels[action] ?? action;
}
</script>

<template>
  <div class="page-stack">
    <PageHeader title="Administrasi" />
    <div class="admin-tabs">
      <button
        :class="{ active: tab === 'parameter' }"
        @click="tab = 'parameter'"
      >
        Parameter</button
      ><button :class="{ active: tab === 'audit' }" @click="tab = 'audit'">
        Audit log
      </button>
    </div>

    <section v-if="tab === 'parameter'" class="admin-grid">
      <article class="panel settings-card">
        <div class="section-title">
          <span class="metric-icon metric-icon--green"
            ><Clock3 :size="20"
          /></span>
          <div>
            <h2>Parameter finansial</h2>
            <p>Nilai baru tidak mengubah transaksi historis.</p>
          </div>
        </div>
        <form class="form-stack" @submit.prevent="save">
          <div class="field-row">
            <label class="field"
              ><span>Simpanan pokok awal</span
              ><input
                v-model.number="form.principalSavings"
                type="number"
                min="0"
                required /></label
            ><label class="field"
              ><span>Simpanan wajib bulanan</span
              ><input
                v-model.number="form.mandatorySavings"
                type="number"
                min="0"
                required
            /></label>
          </div>
          <div class="field-row">
            <label class="field"
              ><span>Provisi pinjaman (%)</span
              ><input
                v-model.number="form.provisionRate"
                type="number"
                min="0"
                step="0.1"
                required /></label
            ><label class="field"
              ><span>Rate bunga default (%)</span
              ><input
                v-model.number="form.annualRate"
                type="number"
                min="0"
                step="0.1"
                required
            /></label>
          </div>
          <label class="field"
            ><span>Tanggal berlaku</span
            ><input
              v-model="form.effectiveDate"
              type="date"
              min="2026-09-14"
              required
          /></label>
          <div class="modal-actions">
            <button
              class="button button--primary"
              type="submit"
              :disabled="saving"
            >
              <Save :size="17" />
              {{ saving ? "Menyimpan…" : "Simpan versi baru" }}
            </button>
          </div>
        </form>
      </article>
      <aside class="panel version-card">
        <p class="eyebrow">Versi aktif</p>
        <h2>{{ admin.parameters.effectiveDate || "Memuat…" }}</h2>
        <StatusPill label="Aktif" tone="success" />
        <dl>
          <div>
            <dt>Simpanan pokok</dt>
            <dd>{{ formatCurrency(admin.parameters.principalSavings) }}</dd>
          </div>
          <div>
            <dt>Simpanan wajib</dt>
            <dd>{{ formatCurrency(admin.parameters.mandatorySavings) }}</dd>
          </div>
          <div>
            <dt>Provisi / bunga</dt>
            <dd>
              {{ admin.parameters.provisionRate }}% /
              {{ admin.parameters.annualRate }}%
            </dd>
          </div>
        </dl>
        <p class="audit-note">
          Perubahan menghasilkan versi baru di SQLite. Perhitungan transaksi
          selalu memilih versi yang berlaku pada tanggal bisnisnya.
        </p>
      </aside>
    </section>

    <section v-else class="panel audit-timeline">
      <div class="panel__header">
        <div>
          <p class="eyebrow">Immutable history</p>
          <h2>Aktivitas backend terbaru</h2>
        </div>
      </div>
      <div class="timeline">
        <article v-for="event in admin.auditEvents" :key="event.id">
          <span><Shield :size="18" /></span>
          <div>
            <strong
              >{{ eventLabel(event.action) }} · {{ event.entityId }}</strong
            >
            <p>{{ event.actor }} · {{ event.entityType }}</p>
            <small>{{ event.createdAt }}</small>
          </div>
        </article>
        <p v-if="admin.auditEvents.length === 0" class="empty-state">
          Belum ada aktivitas operasional baru setelah migrasi.
        </p>
      </div>
    </section>
  </div>
</template>
