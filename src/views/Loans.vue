<script setup lang="ts">
import { ref, onMounted } from "vue";
import { PlusCircle, CreditCard, CheckCircle } from "lucide-vue-next";
import Database from "@tauri-apps/plugin-sql";

interface Loan {
  id: string;
  memberName: string;
  amount: number;
  date: string;
  status: "Berjalan" | "Lunas";
}

const DB_PATH = "sqlite:koperasi.db";

const loans = ref<Loan[]>([]);
const isModalOpen = ref(false);
const newLoanForm = ref({ memberName: "", amount: "" });
const newLoanAmountDisplay = ref("");

async function getDb() {
  return await Database.load(DB_PATH);
}

async function loadLoans() {
  const db = await getDb();
  const rows = await db.select<
    {
      id: string;
      memberName: string;
      amount: number;
      date: string;
      status: string;
    }[]
  >(
    "SELECT id, member_name AS memberName, amount, date, status FROM loans ORDER BY rowid DESC",
  );
  loans.value = rows.map((r) => ({
    ...r,
    status: r.status as "Berjalan" | "Lunas",
  }));
}

onMounted(() => {
  loadLoans();
});

function onAmountInput(e: Event) {
  const target = e.target as HTMLInputElement;
  const rawValue = target.value.replace(/\D/g, "");
  if (!rawValue) {
    newLoanForm.value.amount = "";
    newLoanAmountDisplay.value = "";
    return;
  }
  newLoanForm.value.amount = rawValue;
  newLoanAmountDisplay.value = new Intl.NumberFormat("id-ID").format(
    Number(rawValue),
  );
}

function formatRupiah(amount: number) {
  return new Intl.NumberFormat("id-ID", {
    style: "currency",
    currency: "IDR",
    maximumFractionDigits: 0,
  }).format(amount);
}

async function submitNewLoan() {
  if (newLoanForm.value.memberName && newLoanForm.value.amount) {
    const newId = `PINJ-${Math.floor(100000 + Math.random() * 900000)}`;
    const now = new Date();
    const date = now
      .toLocaleDateString("id-ID", {
        day: "2-digit",
        month: "short",
        year: "numeric",
      })
      .replace(/\./g, "");
    const db = await getDb();
    await db.execute(
      "INSERT INTO loans (id, member_name, amount, date, status) VALUES ($1, $2, $3, $4, $5)",
      [
        newId,
        newLoanForm.value.memberName,
        parseInt(newLoanForm.value.amount),
        date,
        "Berjalan",
      ],
    );
    await loadLoans();
    isModalOpen.value = false;
    newLoanForm.value = { memberName: "", amount: "" };
    newLoanAmountDisplay.value = "";
  }
}

async function markAsPaid(id: string) {
  const db = await getDb();
  await db.execute("UPDATE loans SET status = $1 WHERE id = $2", ["Lunas", id]);
  const loan = loans.value.find((l) => l.id === id);
  if (loan) loan.status = "Lunas";
}
</script>

<template>
  <div class="loan-page">
    <div class="loan-page__stack">
      <header class="loan-page__header">
        <div>
          <h1 class="loan-page__title">Pinjaman</h1>
          <p class="loan-page__subtitle">
            Kelola ajuan dan tagihan pinjaman anggota.
          </p>
        </div>

        <button
          type="button"
          @click="isModalOpen = true"
          class="loan-page__primary-action"
        >
          <PlusCircle class="loan-page__primary-action-icon" />
          <span>Buat Pinjaman Baru</span>
        </button>
      </header>

      <section class="loan-table-card">
        <div class="loan-table-card__scroller">
          <table class="loan-table">
            <thead class="loan-table__head">
              <tr>
                <th scope="col" class="loan-table__heading">ID & Tanggal</th>
                <th scope="col" class="loan-table__heading">Nama Anggota</th>
                <th scope="col" class="loan-table__heading">Jumlah</th>
                <th scope="col" class="loan-table__heading">Status</th>
                <th
                  scope="col"
                  class="loan-table__heading loan-table__heading--right"
                >
                  Aksi
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="loan in loans" :key="loan.id" class="loan-table__row">
                <td class="loan-table__cell loan-table__cell--tight">
                  <div class="loan-table__id">{{ loan.id }}</div>
                  <div class="loan-table__meta">{{ loan.date }}</div>
                </td>
                <td class="loan-table__cell loan-table__cell--tight">
                  <div class="loan-table__member">{{ loan.memberName }}</div>
                </td>
                <td class="loan-table__cell loan-table__cell--tight">
                  <div class="loan-table__amount">
                    {{ formatRupiah(loan.amount) }}
                  </div>
                </td>
                <td class="loan-table__cell loan-table__cell--tight">
                  <span
                    :class="[
                      'loan-status',
                      loan.status === 'Lunas'
                        ? 'loan-status--paid'
                        : 'loan-status--running',
                    ]"
                  >
                    {{ loan.status }}
                  </span>
                </td>
                <td
                  class="loan-table__cell loan-table__cell--right loan-table__cell--tight"
                >
                  <button
                    v-if="loan.status === 'Berjalan'"
                    type="button"
                    @click="markAsPaid(loan.id)"
                    class="loan-table__action"
                  >
                    <CreditCard class="loan-table__action-icon" />
                    Bayar Lunas
                  </button>
                  <span v-else class="loan-table__complete">Terselesaikan</span>
                </td>
              </tr>
              <tr v-if="loans.length === 0">
                <td colspan="5" class="loan-table__empty">
                  Belum ada data pinjaman terdaftar.
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </div>

    <div
      v-if="isModalOpen"
      class="loan-modal"
      @click.self="isModalOpen = false"
    >
      <div class="loan-modal__dialog">
        <h2 class="loan-modal__title">Form Pinjaman Baru</h2>

        <form @submit.prevent="submitNewLoan" class="loan-form">
          <div class="loan-form__field">
            <label class="loan-form__label">Nama Anggota</label>
            <input
              v-model="newLoanForm.memberName"
              type="text"
              required
              class="loan-form__input"
              placeholder="Masukkan nama peminjam"
            />
          </div>
          <div class="loan-form__field">
            <label class="loan-form__label">Jumlah Pinjaman (Rp)</label>
            <input
              :value="newLoanAmountDisplay"
              @input="onAmountInput"
              type="text"
              required
              class="loan-form__input"
              placeholder="Contoh: 5.000.000"
            />
          </div>

          <div class="loan-form__actions">
            <button
              type="button"
              @click="isModalOpen = false"
              class="loan-form__button loan-form__button--secondary"
            >
              Batal
            </button>
            <button
              type="submit"
              class="loan-form__button loan-form__button--primary"
            >
              <CheckCircle class="loan-form__button-icon" />
              Simpan Pinjaman
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>

<style scoped>
.loan-page {
  color: var(--color-text);
}

.loan-page__stack {
  display: flex;
  flex-direction: column;
  gap: 28px;
}

.loan-page__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
}

.loan-page__title {
  margin: 0;
  font-size: clamp(2rem, 4vw, 2.6rem);
  font-weight: 800;
  letter-spacing: -0.05em;
}

.loan-page__subtitle {
  margin: 10px 0 0;
  max-width: 40rem;
  color: var(--color-text-muted);
  font-size: 1rem;
  line-height: 1.7;
}

.loan-page__primary-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  min-height: 58px;
  padding: 0 24px;
  border-radius: 20px;
  background: linear-gradient(135deg, var(--color-primary), #2563eb);
  box-shadow: 0 18px 36px rgba(29, 78, 216, 0.2);
  color: #ffffff;
  cursor: pointer;
  font-size: 1rem;
  font-weight: 800;
  transition:
    transform 160ms ease,
    box-shadow 160ms ease,
    filter 160ms ease;
}

.loan-page__primary-action:hover,
.loan-page__primary-action:focus-visible {
  transform: translateY(-1px);
  box-shadow: 0 22px 40px rgba(29, 78, 216, 0.24);
  filter: saturate(1.06);
  outline: none;
}

.loan-page__primary-action-icon {
  width: 22px;
  height: 22px;
}

.loan-table-card {
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: 28px;
  background: var(--color-surface-strong);
  box-shadow: var(--shadow-soft);
}

.loan-table-card__scroller {
  overflow-x: auto;
}

.loan-table {
  width: 100%;
  min-width: 760px;
  border-collapse: collapse;
}

.loan-table__head {
  background: linear-gradient(180deg, #f8fbff, #eef4fb);
}

.loan-table__heading {
  padding: 24px 28px;
  border-bottom: 1px solid var(--color-border);
  color: var(--color-text-muted);
  font-size: 0.95rem;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-align: left;
  text-transform: uppercase;
}

.loan-table__heading--right {
  text-align: right;
}

.loan-table__row {
  transition: background-color 160ms ease;
}

.loan-table__row:hover {
  background: rgba(255, 237, 213, 0.48);
}

.loan-table__cell {
  padding: 24px 28px;
  border-bottom: 1px solid rgba(215, 224, 234, 0.8);
  vertical-align: middle;
}

.loan-table__cell--tight {
  white-space: nowrap;
}

.loan-table__cell--right {
  text-align: right;
}

.loan-table__id,
.loan-table__member,
.loan-table__amount {
  font-size: 1.02rem;
  font-weight: 700;
}

.loan-table__id,
.loan-table__member {
  color: var(--color-text);
}

.loan-table__amount {
  color: var(--color-primary-strong);
}

.loan-table__meta {
  margin-top: 6px;
  color: var(--color-text-soft);
  font-size: 0.92rem;
}

.loan-status {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 96px;
  padding: 10px 16px;
  border: 1px solid transparent;
  border-radius: 999px;
  font-size: 0.92rem;
  font-weight: 800;
}

.loan-status--running {
  border-color: rgba(245, 158, 11, 0.18);
  background: rgba(254, 243, 199, 0.8);
  color: #92400e;
}

.loan-status--paid {
  border-color: rgba(22, 163, 74, 0.18);
  background: rgba(220, 252, 231, 0.88);
  color: #166534;
}

.loan-table__action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  min-height: 48px;
  padding: 0 18px;
  border-radius: 16px;
  background: rgba(29, 78, 216, 0.94);
  color: #ffffff;
  cursor: pointer;
  font-size: 0.95rem;
  font-weight: 700;
  transition:
    transform 160ms ease,
    background-color 160ms ease;
}

.loan-table__action:hover,
.loan-table__action:focus-visible {
  background: var(--color-primary-strong);
  transform: translateY(-1px);
  outline: none;
}

.loan-table__action-icon {
  width: 18px;
  height: 18px;
}

.loan-table__complete {
  color: var(--color-text-soft);
  font-size: 0.95rem;
  font-weight: 700;
}

.loan-table__empty {
  padding: 48px 28px;
  color: var(--color-text-soft);
  font-size: 1rem;
  font-weight: 600;
  text-align: center;
}

.loan-modal {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: rgba(23, 32, 51, 0.44);
  backdrop-filter: blur(12px);
}

.loan-modal__dialog {
  width: min(100%, 680px);
  padding: 32px;
  border: 1px solid rgba(255, 255, 255, 0.78);
  border-radius: 30px;
  background: rgba(255, 255, 255, 0.96);
  box-shadow: var(--shadow-card);
}

.loan-modal__title {
  margin: 0 0 24px;
  padding-bottom: 18px;
  border-bottom: 1px solid rgba(215, 224, 234, 0.78);
  font-size: 1.75rem;
  font-weight: 800;
  letter-spacing: -0.04em;
}

.loan-form {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.loan-form__label {
  display: block;
  margin-bottom: 10px;
  font-size: 0.98rem;
  font-weight: 700;
}

.loan-form__input {
  width: 100%;
  padding: 16px 18px;
  border: 1px solid var(--color-border);
  border-radius: 18px;
  background: var(--color-surface-muted);
  color: var(--color-text);
  font-size: 1rem;
  transition:
    border-color 160ms ease,
    background-color 160ms ease,
    box-shadow 160ms ease;
}

.loan-form__input::placeholder {
  color: var(--color-text-soft);
}

.loan-form__input:hover {
  border-color: var(--color-border-strong);
}

.loan-form__input:focus-visible {
  border-color: var(--color-accent);
  outline: none;
  background: var(--color-surface-strong);
  box-shadow: 0 0 0 4px rgba(249, 115, 22, 0.14);
}

.loan-form__actions {
  display: flex;
  gap: 14px;
  padding-top: 8px;
}

.loan-form__button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  flex: 1;
  min-height: 54px;
  padding: 0 18px;
  border-radius: 18px;
  cursor: pointer;
  font-size: 1rem;
  font-weight: 800;
  transition:
    transform 160ms ease,
    box-shadow 160ms ease,
    background-color 160ms ease;
}

.loan-form__button:hover,
.loan-form__button:focus-visible {
  transform: translateY(-1px);
  outline: none;
}

.loan-form__button--secondary {
  border: 1px solid var(--color-border);
  background: rgba(241, 245, 249, 0.92);
  color: var(--color-text-muted);
}

.loan-form__button--secondary:hover,
.loan-form__button--secondary:focus-visible {
  background: rgba(226, 232, 240, 0.96);
}

.loan-form__button--primary {
  background: linear-gradient(135deg, var(--color-primary), #2563eb);
  box-shadow: 0 18px 36px rgba(29, 78, 216, 0.22);
  color: #ffffff;
}

.loan-form__button--primary:hover,
.loan-form__button--primary:focus-visible {
  box-shadow: 0 22px 40px rgba(29, 78, 216, 0.26);
}

.loan-form__button-icon {
  width: 20px;
  height: 20px;
}

@media (max-width: 900px) {
  .loan-page__header {
    flex-direction: column;
    align-items: stretch;
  }

  .loan-page__primary-action {
    width: 100%;
  }
}

@media (max-width: 640px) {
  .loan-form__actions {
    flex-direction: column;
  }

  .loan-modal {
    padding: 16px;
  }

  .loan-modal__dialog {
    padding: 24px 18px;
    border-radius: 24px;
  }
}
</style>
