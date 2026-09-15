<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import {
  Calculator,
  CalendarX2,
  Download,
  Plus,
  Search,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import StatusPill from "../components/StatusPill.vue";
import UiModal from "../components/UiModal.vue";
import {
  formatCurrency,
  type LoanPreview,
  useKoperasiStore,
} from "../store/koperasi";

const {
  members,
  selectedYear,
  yearLoans,
  yearTotals,
  createLoan,
  disburseLoan,
  previewLoan,
  notify,
  refresh,
} = useKoperasiStore();
const query = ref("");
const status = ref("Semua status");
const open = ref(false);
const disbursing = ref("");
const loanHasData = computed(() => yearLoans.value.length > 0);
const form = reactive({
  memberId: "",
  plafond: 10_000_000,
  tenor: 24,
  interestType: "Menurun" as "Menurun" | "Flat",
});
const filtered = computed(() =>
  yearLoans.value.filter(
    (loan) =>
      (status.value === "Semua status" || loan.status === status.value) &&
      `${loan.memberName} ${loan.id}`
        .toLowerCase()
        .includes(query.value.toLowerCase()),
  ),
);
const preview = ref<LoanPreview | null>(null);
const activeLoanCount = computed(
  () => yearLoans.value.filter((loan) => loan.status === "Berjalan").length,
);
const reviewLoans = computed(() =>
  yearLoans.value.filter((loan) => loan.status === "Perlu review"),
);
const reviewBalance = computed(() =>
  reviewLoans.value.reduce((sum, loan) => sum + loan.balance, 0),
);
const annualPlafond = computed(() =>
  yearLoans.value.reduce((sum, loan) => sum + loan.plafond, 0),
);
watch(
  form,
  async () => {
    try {
      preview.value = await previewLoan({ ...form });
    } catch {
      preview.value = null;
    }
  },
  { deep: true, immediate: true },
);
async function submit() {
  if (!form.memberId || form.plafond <= 0) return;
  const saved = await createLoan({ ...form });
  if (!saved) return;
  open.value = false;
}
async function handleDisbursement(loanId: string) {
  disbursing.value = loanId;
  await disburseLoan(loanId);
  disbursing.value = "";
}
</script>

<template>
  <div class="page-stack">
    <PageHeader title="Pinjaman" :refresh="refresh">
      <template #actions
        ><button
          class="button button--secondary"
          type="button"
          @click="
            notify(
              'Ekspor disiapkan',
              'Laporan portofolio akan diekspor.',
              'info',
            )
          "
        >
          <Download :size="18" /> Ekspor</button
        ><button
          class="button button--primary"
          type="button"
          @click="open = true"
        >
          <Plus :size="18" /> Buat pinjaman
        </button></template
      >
    </PageHeader>
    <section v-if="!loanHasData" class="panel year-empty-state">
      <CalendarX2 :size="36" />
      <strong>Belum ada data pinjaman untuk {{ selectedYear }}</strong>
      <p>Pilih tahun lain untuk melihat kontrak dan portofolio pinjaman.</p>
    </section>
    <template v-else>
      <section class="mini-metrics">
        <article>
          <span>Outstanding</span
          ><strong>{{ formatCurrency(yearTotals.loanPortfolio, true) }}</strong
          ><small>{{ activeLoanCount }} kontrak berjalan</small>
        </article>
        <article>
          <span>Pencairan {{ selectedYear }}</span
          ><strong>{{ formatCurrency(annualPlafond, true) }}</strong
          ><small>{{ yearLoans.length }} pinjaman baru</small>
        </article>
        <article>
          <span>Perlu review</span
          ><strong>{{ formatCurrency(reviewBalance, true) }}</strong
          ><small class="text-warning"
            >{{ reviewLoans.length }} kontrak perlu ditindaklanjuti</small
          >
        </article>
      </section>
      <section class="panel table-panel">
        <div class="toolbar">
          <label class="search-field"
            ><Search :size="18" /><input
              v-model="query"
              placeholder="Cari anggota atau ID pinjaman..." /></label
          ><select v-model="status" class="select-control">
            <option>Semua status</option>
            <option>Draf</option>
            <option>Berjalan</option>
            <option>Perlu review</option>
            <option>Lunas</option>
          </select>
        </div>
        <div class="data-table-wrap">
          <table class="data-table">
            <thead>
              <tr>
                <th>Kontrak</th>
                <th>Anggota</th>
                <th>Plafond</th>
                <th>Saldo pokok</th>
                <th>Skema</th>
                <th>Jatuh tempo</th>
                <th>Status</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="loan in filtered" :key="loan.id">
                <td>
                  <strong>{{ loan.id }}</strong
                  ><small class="cell-sub"
                    >Realisasi {{ loan.realizationDate }}</small
                  >
                </td>
                <td>
                  <strong>{{ loan.memberName }}</strong
                  ><small class="cell-sub">{{
                    loan.memberId || "Belum dipadankan"
                  }}</small>
                </td>
                <td class="num-cell">
                  {{ loan.plafond ? formatCurrency(loan.plafond) : "—" }}
                </td>
                <td class="num-cell">
                  <strong>{{ formatCurrency(loan.balance) }}</strong>
                </td>
                <td>
                  {{ loan.interestType
                  }}<small class="cell-sub"
                    >{{ loan.rate }}% / tahun · {{ loan.tenor }} bln</small
                  >
                </td>
                <td>{{ loan.dueDate }}</td>
                <td>
                  <StatusPill
                    :label="loan.status"
                    :tone="
                      loan.status === 'Berjalan'
                        ? 'success'
                        : loan.status === 'Perlu review'
                          ? 'warning'
                          : loan.status === 'Draf'
                            ? 'info'
                            : 'neutral'
                    "
                  />
                </td>
                <td>
                  <button
                    v-if="loan.status === 'Draf'"
                    class="row-action"
                    type="button"
                    :disabled="disbursing === loan.id"
                    @click="handleDisbursement(loan.id)"
                  >
                    {{ disbursing === loan.id ? "Memproses…" : "Cairkan" }}
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </template>
    <UiModal
      :open="open"
      size="lg"
      title="Buat pinjaman baru"
      description="Parameter default mengikuti versi aktif per 1 Januari 2026."
      @close="open = false"
    >
      <form class="form-stack" @submit.prevent="submit">
        <label class="field"
          ><span>Anggota</span
          ><select v-model="form.memberId" required>
            <option value="" disabled>Pilih anggota</option>
            <option
              v-for="member in members"
              :key="member.id"
              :value="member.id"
            >
              {{ member.name }} · {{ member.memberNumber }}
            </option>
          </select></label
        >
        <div class="field-row">
          <label class="field"
            ><span>Plafond pinjaman</span
            ><input
              v-model.number="form.plafond"
              type="number"
              min="100000"
              step="100000"
              required /></label
          ><label class="field"
            ><span>Tenor</span
            ><select v-model.number="form.tenor">
              <option :value="10">10 bulan</option>
              <option :value="12">12 bulan</option>
              <option :value="15">15 bulan</option>
              <option :value="24">24 bulan</option>
              <option :value="36">36 bulan</option>
            </select></label
          >
        </div>
        <div class="field-row">
          <label class="field"
            ><span>Jenis bunga</span
            ><select v-model="form.interestType">
              <option>Menurun</option>
              <option>Flat</option>
            </select></label
          ><label class="field"
            ><span>Rate tahunan</span
            ><input :value="`${preview?.annualRate ?? 0}%`" disabled
          /></label>
        </div>
        <div class="calculation-preview">
          <div class="calculation-preview__title">
            <Calculator :size="19" /> Preview bulan pertama
          </div>
          <dl>
            <div>
              <dt>Angsuran pokok</dt>
              <dd>{{ formatCurrency(preview?.principalInstallment ?? 0) }}</dd>
            </div>
            <div>
              <dt>Estimasi bunga</dt>
              <dd>{{ formatCurrency(preview?.firstInterest ?? 0) }}</dd>
            </div>
            <div>
              <dt>Provisi 1%</dt>
              <dd>{{ formatCurrency(preview?.provision ?? 0) }}</dd>
            </div>
            <div class="is-total">
              <dt>Total tagihan pertama</dt>
              <dd>{{ formatCurrency(preview?.firstTotal ?? 0) }}</dd>
            </div>
          </dl>
        </div>
        <div class="modal-actions">
          <button
            class="button button--secondary"
            type="button"
            @click="open = false"
          >
            Batal</button
          ><button class="button button--primary" type="submit">
            Simpan sebagai draf
          </button>
        </div>
      </form>
    </UiModal>
  </div>
</template>
