<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import {
  ArrowDownToLine,
  ArrowUpFromLine,
  CalendarX2,
  Coins,
  Search,
} from "lucide-vue-next";
import PageHeader from "../components/PageHeader.vue";
import StatusPill from "../components/StatusPill.vue";
import UiModal from "../components/UiModal.vue";
import {
  formatCurrency,
  MemberStatus,
  SavingsAccountType,
  SavingsMovement,
  useKoperasiStore,
} from "../store/koperasi";

const {
  members,
  selectedYear,
  yearMembers,
  yearTotals,
  yearHasData,
  admin,
  loadAdminState,
  postSavingsTransaction,
  refresh,
} = useKoperasiStore();
const query = ref("");
const open = ref(false);
const form = reactive({
  memberId: "",
  accountType: SavingsAccountType.Mandatory,
  movement: SavingsMovement.Deposit,
  amount: 50_000,
  reference: "",
});
const filtered = computed(() =>
  yearMembers.value.filter((member) =>
    member.name.toLowerCase().includes(query.value.toLowerCase()),
  ),
);
const principalTotal = computed(() =>
  yearMembers.value.reduce((sum, member) => sum + member.principalSavings, 0),
);
const mandatoryTotal = computed(() =>
  yearMembers.value.reduce((sum, member) => sum + member.mandatorySavings, 0),
);
const voluntaryTotal = computed(() =>
  yearMembers.value.reduce((sum, member) => sum + member.voluntarySavings, 0),
);
onMounted(loadAdminState);
async function refreshPage() {
  await Promise.all([refresh(), loadAdminState()]);
}
function startMovement(movement: SavingsMovement) {
  Object.assign(form, {
    memberId: "",
    accountType:
      movement === SavingsMovement.Withdrawal
        ? SavingsAccountType.Voluntary
        : SavingsAccountType.Mandatory,
    movement,
    amount:
      movement === SavingsMovement.Deposit
        ? admin.parameters.mandatorySavings || 50_000
        : 50_000,
    reference: "",
  });
  open.value = true;
}
async function submit() {
  if (!form.memberId || form.amount <= 0) return;
  if (!(await postSavingsTransaction({ ...form }))) return;
  open.value = false;
}
</script>
<template>
  <div class="page-stack">
    <PageHeader title="Simpanan" :refresh="refreshPage"
      ><template #actions
        ><button
          class="button button--secondary"
          @click="startMovement(SavingsMovement.Withdrawal)"
        >
          <ArrowUpFromLine :size="18" /> Penarikan</button
        ><button
          class="button button--primary"
          @click="startMovement(SavingsMovement.Deposit)"
        >
          <ArrowDownToLine :size="18" /> Setoran
        </button></template
      ></PageHeader
    >
    <section v-if="!yearHasData" class="panel year-empty-state">
      <CalendarX2 :size="36" />
      <strong>Belum ada data simpanan untuk {{ selectedYear }}</strong>
      <p>Pilih tahun lain untuk melihat saldo dan rekening simpanan.</p>
    </section>
    <template v-else>
      <section class="savings-summary">
        <article class="savings-hero">
          <span><Coins :size="24" /></span>
          <p>Total simpanan anggota</p>
          <strong>{{ formatCurrency(yearTotals.savings) }}</strong
          ><small>Saldo dan mutasi tahun {{ selectedYear }}</small>
        </article>
        <article>
          <p>Simpanan pokok</p>
          <strong>{{ formatCurrency(principalTotal, true) }}</strong
          ><span>{{ yearTotals.members }} rekening</span>
        </article>
        <article>
          <p>Simpanan wajib</p>
          <strong>{{ formatCurrency(mandatoryTotal, true) }}</strong
          ><span>Saldo ledger</span>
        </article>
        <article>
          <p>Manasuka</p>
          <strong>{{ formatCurrency(voluntaryTotal, true) }}</strong
          ><span>Dapat ditarik</span>
        </article>
      </section>
      <section class="panel table-panel">
        <div class="toolbar">
          <label class="search-field"
            ><Search :size="18" /><input
              v-model="query"
              placeholder="Cari anggota..."
          /></label>
          <div class="toolbar__meta">Tahun {{ selectedYear }}</div>
        </div>
        <div class="data-table-wrap">
          <table class="data-table">
            <thead>
              <tr>
                <th>Anggota</th>
                <th>Pokok</th>
                <th>Wajib</th>
                <th>Manasuka</th>
                <th>Total saldo</th>
                <th>Status rekening</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="member in filtered" :key="member.id">
                <td>
                  <strong>{{ member.name }}</strong
                  ><small class="cell-sub">{{ member.memberNumber }}</small>
                </td>
                <td class="num-cell">
                  {{ formatCurrency(member.principalSavings) }}
                </td>
                <td class="num-cell">
                  {{ formatCurrency(member.mandatorySavings) }}
                </td>
                <td class="num-cell">
                  {{ formatCurrency(member.voluntarySavings) }}
                </td>
                <td class="num-cell">
                  <strong>{{ formatCurrency(member.savings) }}</strong>
                </td>
                <td>
                  <StatusPill :label="MemberStatus.Active" tone="success" />
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </template>
    <UiModal
      :open="open"
      :title="`${form.movement} simpanan`"
      description="Posting akan memperbarui sub-ledger, kas, dan audit trail dalam satu transaksi database."
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
        <label class="field"
          ><span>Jenis simpanan</span
          ><select
            v-model="form.accountType"
            :disabled="form.movement === SavingsMovement.Withdrawal"
          >
            <option :value="SavingsAccountType.Principal">Pokok</option>
            <option :value="SavingsAccountType.Mandatory">Wajib</option>
            <option :value="SavingsAccountType.Voluntary">Manasuka</option>
          </select></label
        >
        <label class="field"
          ><span>Nominal</span
          ><input
            v-model.number="form.amount"
            type="number"
            min="1000"
            step="1000"
            required
        /></label>
        <label class="field"
          ><span>Nomor referensi (opsional)</span
          ><input
            v-model="form.reference"
            placeholder="Dibuat otomatis bila kosong"
        /></label>
        <div class="modal-actions">
          <button
            class="button button--secondary"
            type="button"
            @click="open = false"
          >
            Batal</button
          ><button class="button button--primary" type="submit">
            Post {{ form.movement.toLowerCase() }}
          </button>
        </div>
      </form>
    </UiModal>
  </div>
</template>
