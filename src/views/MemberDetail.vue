<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useRoute } from "vue-router";
import {
  ArrowLeft,
  BadgeInfo,
  Landmark,
  UserRound,
  WalletCards,
} from "lucide-vue-next";
import DateInput from "../components/DateInput.vue";
import PageHeader from "../components/PageHeader.vue";
import StatusPill from "../components/StatusPill.vue";
import {
  formatCurrency,
  formatPeriod,
  formatPreviousPeriod,
  SavingsAccountType,
  TransactionDirection,
  useKoperasiStore,
  useMonthlyLedger,
} from "../store/koperasi";

const route = useRoute();
const {
  members,
  loans,
  savingsBalances,
  transactions,
  refresh,
  memberNumberOwner,
  updateMember,
} = useKoperasiStore();

const member = computed(() =>
  members.find((item) => item.id === String(route.params.id)),
);
const saving = ref(false);
const draft = reactive({ name: "", memberNumber: "", joinedAt: "" });
// Imported members carry a placeholder such as "Migrasi 2026" instead of a date.
const storedJoinedAt = computed(() => {
  const value = member.value?.joinedAt ?? "";
  return /^\d{4}-\d{2}-\d{2}$/.test(value) ? value : "";
});
function resetDraft() {
  draft.name = member.value?.name ?? "";
  draft.memberNumber = member.value?.memberNumber ?? "";
  draft.joinedAt = storedJoinedAt.value;
}
watch(
  () => [
    member.value?.id,
    member.value?.name,
    member.value?.memberNumber,
    member.value?.joinedAt,
  ],
  resetDraft,
  { immediate: true },
);
const memberNumberTaken = computed(() =>
  memberNumberOwner(draft.memberNumber, member.value?.id),
);
const dirty = computed(
  () =>
    Boolean(member.value) &&
    (draft.name.trim() !== member.value?.name ||
      draft.memberNumber.trim() !== member.value?.memberNumber ||
      draft.joinedAt !== storedJoinedAt.value),
);
async function saveMember() {
  if (!member.value || !dirty.value) return;
  if (!draft.name.trim() || !draft.memberNumber.trim()) return;
  if (memberNumberTaken.value) return;
  saving.value = true;
  try {
    await updateMember(
      member.value.id,
      draft.name,
      draft.memberNumber,
      draft.joinedAt,
    );
  } finally {
    saving.value = false;
  }
}

const memberLoans = computed(() =>
  member.value
    ? loans.filter((loan) => loan.memberId === member.value?.id)
    : [],
);
const savingsOf = (accountType: SavingsAccountType) =>
  savingsBalances.find(
    (item) =>
      item.memberId === member.value?.id && item.accountType === accountType,
  )?.balance ?? 0;
const savingsBreakdown = computed(() => [
  { label: "Simpanan pokok", amount: member.value?.principalSavings ?? 0 },
  { label: "Simpanan wajib", amount: savingsOf(SavingsAccountType.Mandatory) },
  {
    label: "Simpanan manasuka",
    amount: savingsOf(SavingsAccountType.Voluntary),
  },
]);

// Arrears come from the monthly ledger of the selected year: the current
// month when it is in that ledger, otherwise its latest month.
const { monthlyLedger } = useMonthlyLedger();
const today = new Date();
const thisPeriod = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}`;
const arrearsPeriod = computed(() => {
  const periods = monthlyLedger.value.periods.filter(
    (period) => period <= thisPeriod,
  );
  return periods.length ? periods.reduce((a, b) => (a > b ? a : b)) : "";
});
type ArrearsPart = "Principal" | "Interest";

const memberLedgerRows = computed(() => {
  const ids = new Set(memberLoans.value.map((loan) => loan.id));
  return monthlyLedger.value.loans
    .filter((row) => ids.has(row.loanId) && row.period <= arrearsPeriod.value)
    .sort((a, b) => a.period.localeCompare(b.period));
});

/** Unpaid amount per month, oldest first: the arrears are matched against
 * the latest installments, walking back month by month. Arrears reaching
 * past the selected year are reported as "s/d" the month before it. */
function unpaidMonths(part: ArrearsPart) {
  const byPeriod = new Map<string, number>();
  let earlier = 0;
  let firstPeriod = "";
  for (const loan of memberLoans.value) {
    const rows = memberLedgerRows.value.filter((row) => row.loanId === loan.id);
    const latest = rows.find((row) => row.period === arrearsPeriod.value);
    let remaining = latest?.[`arrears${part}`] ?? 0;
    for (let i = rows.length - 1; i >= 0 && remaining > 0; i--) {
      const unpaid = Math.min(rows[i][`scheduled${part}`], remaining);
      if (unpaid <= 0) continue;
      byPeriod.set(
        rows[i].period,
        (byPeriod.get(rows[i].period) ?? 0) + unpaid,
      );
      remaining -= unpaid;
    }
    if (remaining > 0 && rows.length) {
      earlier += remaining;
      if (!firstPeriod || rows[0].period < firstPeriod)
        firstPeriod = rows[0].period;
    }
  }
  const months = [...byPeriod]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([period, amount]) => ({ label: formatPeriod(period), amount }));
  if (earlier > 0)
    months.unshift({
      label: `s/d ${formatPreviousPeriod(firstPeriod)}`,
      amount: earlier,
    });
  return months;
}

const loanSummary = computed(() => {
  const latest = memberLedgerRows.value.filter(
    (row) => row.period === arrearsPeriod.value,
  );
  const per = arrearsPeriod.value
    ? `Per ${formatPeriod(arrearsPeriod.value)}`
    : "";
  const arrears = (part: ArrearsPart) => {
    const amount = latest.reduce((sum, row) => sum + row[`arrears${part}`], 0);
    return {
      amount,
      note: amount < 0 ? [per, "Lebih bayar"].filter(Boolean).join(" · ") : per,
      months: amount > 0 ? unpaidMonths(part) : [],
    };
  };
  return [
    {
      label: "Saldo pinjaman",
      amount: memberLoans.value.reduce((sum, loan) => sum + loan.balance, 0),
      note: "",
      months: [],
    },
    { label: "Tunggakan pokok", ...arrears("Principal") },
    { label: "Tunggakan bunga", ...arrears("Interest") },
  ];
});

// "Aktivitas terbaru" is hidden for now.
const showRecentActivity = false;
const memberTransactions = computed(() => {
  if (!member.value) return [];
  const { memberNumber } = member.value;
  const memberName = member.value.name.toLocaleLowerCase("id-ID");
  return transactions
    .filter((item) =>
      memberNumber
        ? item.memberNumber === memberNumber
        : item.memberName.toLocaleLowerCase("id-ID") === memberName,
    )
    .slice(0, 5);
});
</script>

<template>
  <div class="page-stack">
    <PageHeader :title="member?.name ?? 'Detail anggota'" :refresh="refresh">
      <template #actions>
        <RouterLink class="button button--secondary" to="/members">
          <ArrowLeft :size="18" /> Kembali ke anggota
        </RouterLink>
      </template>
    </PageHeader>

    <section v-if="member" class="member-detail member-detail--page panel">
      <section class="member-detail__identity">
        <span class="member-detail__avatar"><UserRound :size="28" /></span>
        <form class="member-edit-form" @submit.prevent="saveMember">
          <label class="field"
            ><span>ID anggota</span
            ><input
              v-model="draft.memberNumber"
              aria-label="ID anggota"
              required
              autocomplete="off"
              :aria-invalid="Boolean(memberNumberTaken)"
          /></label>
          <label class="field"
            ><span>Nama lengkap</span
            ><input
              v-model="draft.name"
              aria-label="Nama lengkap"
              required
              autocomplete="off"
          /></label>
          <div class="field">
            <span>Tanggal bergabung</span>
            <DateInput
              v-model="draft.joinedAt"
              aria-label="Tanggal bergabung"
            />
          </div>
          <div class="member-edit-form__actions">
            <button
              class="button button--secondary"
              type="button"
              :disabled="!dirty || saving"
              @click="resetDraft"
            >
              Batal
            </button>
            <button
              class="button button--primary"
              type="submit"
              :disabled="!dirty || saving"
            >
              Simpan
            </button>
          </div>
          <p v-if="memberNumberTaken" class="field-error" role="alert">
            ID anggota {{ draft.memberNumber.trim() }} sudah dipakai oleh
            {{ memberNumberTaken.name }}.
          </p>
        </form>
      </section>

      <section class="member-detail__section">
        <div class="member-detail__section-title">
          <WalletCards :size="17" />
          <h3>Rincian simpanan</h3>
        </div>
        <div class="member-detail__breakdown">
          <div v-for="item in savingsBreakdown" :key="item.label">
            <span>{{ item.label }}</span>
            <strong>{{ formatCurrency(item.amount) }}</strong>
          </div>
        </div>
      </section>

      <section class="member-detail__section">
        <div class="member-detail__section-title">
          <Landmark :size="17" />
          <h3>Pinjaman</h3>
        </div>
        <div v-if="memberLoans.length" class="loan-summary">
          <div v-for="item in loanSummary" :key="item.label">
            <div class="loan-summary__row">
              <div>
                <span>{{ item.label }}</span>
                <small v-if="item.note">{{ item.note }}</small>
              </div>
              <strong>{{ formatCurrency(item.amount) }}</strong>
            </div>
            <ul v-if="item.months.length" class="loan-summary__months">
              <li v-for="month in item.months" :key="month.label">
                <span>{{ month.label }}</span>
                <span>{{ formatCurrency(month.amount) }}</span>
              </li>
            </ul>
          </div>
        </div>
        <div v-if="memberLoans.length" class="member-detail__list">
          <div v-for="loan in memberLoans" :key="loan.id">
            <div>
              <strong>{{ loan.id }}</strong>
              <small
                >{{ loan.loanType ? `${loan.loanType} · ` : ""
                }}{{ loan.interestType }} · {{ loan.tenor }} bulan</small
              >
            </div>
            <div class="member-detail__list-value">
              <strong>{{ formatCurrency(loan.balance) }}</strong>
              <StatusPill
                v-if="loan.balance < 0"
                label="Lebih bayar"
                tone="warning"
              />
              <StatusPill v-else-if="loan.balance === 0" label="Lunas" />
            </div>
          </div>
        </div>
        <p v-else class="empty-inline">Belum ada pinjaman untuk anggota ini.</p>
      </section>

      <section v-if="showRecentActivity" class="member-detail__section">
        <div class="member-detail__section-title">
          <BadgeInfo :size="17" />
          <h3>Aktivitas terbaru</h3>
        </div>
        <div v-if="memberTransactions.length" class="member-detail__list">
          <div v-for="item in memberTransactions" :key="item.id">
            <div>
              <strong>{{ item.description }}</strong>
              <small>{{ item.date }} · {{ item.reference }}</small>
            </div>
            <strong
              :class="
                item.direction === TransactionDirection.In
                  ? 'amount-in'
                  : 'amount-out'
              "
            >
              {{ item.direction === TransactionDirection.In ? "+" : "−"
              }}{{ formatCurrency(item.amount) }}
            </strong>
          </div>
        </div>
        <p v-else class="empty-inline">Belum ada aktivitas transaksi.</p>
      </section>
    </section>

    <section v-else class="panel member-detail__not-found">
      <UserRound :size="32" />
      <strong>Anggota tidak ditemukan</strong>
      <p>Data anggota pada alamat ini tidak tersedia.</p>
      <RouterLink class="button button--primary" to="/members">
        Kembali ke daftar anggota
      </RouterLink>
    </section>
  </div>
</template>

<style scoped>
.amount-in {
  color: var(--green) !important;
}
.amount-out {
  color: var(--red) !important;
}
.member-detail--page {
  padding: 24px;
}
.member-detail__identity {
  display: flex;
  align-items: center;
  gap: 14px;
}
.member-detail__avatar {
  flex: 0 0 auto;
  width: 56px;
  height: 56px;
  display: grid;
  place-items: center;
  color: var(--green);
  background: var(--green-soft);
}
.member-detail__identity small,
.member-detail__list small {
  color: var(--soft);
  font-size: 9.5px;
}
.member-detail__identity h3 {
  margin: 3px 0 6px;
  font-size: 17px;
}
.member-detail__section {
  display: grid;
  gap: 10px;
}
.member-detail__section-title {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--green);
}
.member-detail__section-title h3 {
  margin: 0;
  color: var(--ink);
  font-size: 12px;
}
.loan-summary {
  border: 1px solid var(--line);
}
.loan-summary > div {
  padding: 11px 13px;
}
.loan-summary > div + div {
  border-top: 1px solid var(--line);
}
.loan-summary__row,
.loan-summary__months li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}
.loan-summary__row > div {
  display: grid;
  gap: 3px;
}
.loan-summary__row span {
  color: var(--muted);
  font-size: 10.5px;
}
.loan-summary__row strong {
  font-size: 11px;
}
.loan-summary small {
  color: var(--soft);
  font-size: 9.5px;
}
.loan-summary__months {
  display: grid;
  gap: 6px;
  margin: 10px 0 0;
  padding: 0 0 0 14px;
  border-left: 2px solid var(--line);
  list-style: none;
  font-size: 10.5px;
}
.member-detail__breakdown {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  border: 1px solid var(--line);
}
.member-detail__breakdown > div {
  display: grid;
  gap: 5px;
  padding: 12px 14px;
}
.member-detail__breakdown > div + div {
  border-left: 1px solid var(--line);
}
.member-detail__breakdown strong {
  font-size: 11px;
}
.member-detail__list {
  border: 1px solid var(--line);
}
.member-detail__list > div {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 11px 13px;
}
.member-detail__list > div + div {
  border-top: 1px solid var(--line);
}
.member-detail__list > div > div:first-child {
  display: grid;
  gap: 3px;
}
.member-detail__list strong {
  font-size: 10.5px;
}
.member-detail__list-value {
  display: flex;
  align-items: center;
  gap: 10px;
  text-align: right;
}
.member-detail__not-found {
  min-height: 300px;
  display: grid;
  place-items: center;
  align-content: center;
  gap: 8px;
  padding: 32px;
  color: var(--soft);
  text-align: center;
}
.member-detail__not-found strong {
  color: var(--ink);
}
.member-detail__not-found p {
  margin: 0 0 10px;
  color: var(--muted);
  font-size: 11px;
}
.empty-inline {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 8px;
  padding: 22px;
  color: var(--green);
}
.member-detail .empty-inline {
  margin: 0;
  padding: 14px;
  border: 1px dashed var(--line-strong);
  color: var(--soft);
  font-size: 10px;
  text-align: center;
}
@media (max-width: 540px) {
  .member-detail__breakdown {
    grid-template-columns: 1fr;
  }
  .member-detail__breakdown > div + div {
    border-top: 1px solid var(--line);
    border-left: 0;
  }
  .member-detail__list > div {
    align-items: flex-start;
  }
  .member-detail__list-value {
    align-items: flex-end;
    flex-direction: column;
  }
}
/* ID + name on the first row, join date + actions on the second, so each
   row's labels line up despite the date picker's Hari/Bulan/Tahun captions. */
.member-edit-form {
  flex: 1;
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 2fr);
  align-items: end;
  gap: 12px 10px;
}
.member-edit-form__actions {
  display: flex;
  justify-self: end;
  gap: 8px;
}
@media (max-width: 720px) {
  .member-edit-form {
    grid-template-columns: 1fr;
  }
}
</style>
