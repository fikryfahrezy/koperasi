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
  <div>
    <div class="space-y-8">
      <header
        class="flex flex-col sm:flex-row sm:items-center justify-between gap-4"
      >
        <div>
          <h1 class="text-3xl sm:text-4xl font-bold text-gray-900 mb-2">
            Pinjaman
          </h1>
          <p class="text-lg sm:text-xl text-gray-600">
            Kelola ajuan dan tagihan pinjaman anggota.
          </p>
        </div>

        <button
          @click="isModalOpen = true"
          class="w-full sm:w-auto bg-blue-600 text-white px-6 sm:px-8 py-3 sm:py-4 rounded-xl shadow-lg border-2 border-transparent hover:bg-blue-700 hover:shadow-xl transition flex items-center justify-center space-x-3 text-lg sm:text-xl font-bold outline-none focus-visible:border-blue-300"
        >
          <PlusCircle class="w-6 h-6 sm:w-8 sm:h-8" />
          <span>Buat Pinjaman Baru</span>
        </button>
      </header>

      <!-- Tabel / Daftar Pinjaman -->
      <div
        class="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden mt-8"
      >
        <div class="overflow-x-auto">
          <table class="min-w-full divide-y divide-gray-200">
            <thead class="bg-gray-100">
              <tr>
                <th
                  scope="col"
                  class="px-8 py-6 text-left text-xl font-bold text-gray-700"
                >
                  ID & Tanggal
                </th>
                <th
                  scope="col"
                  class="px-8 py-6 text-left text-xl font-bold text-gray-700"
                >
                  Nama Anggota
                </th>
                <th
                  scope="col"
                  class="px-8 py-6 text-left text-xl font-bold text-gray-700"
                >
                  Jumlah
                </th>
                <th
                  scope="col"
                  class="px-8 py-6 text-left text-xl font-bold text-gray-700"
                >
                  Status
                </th>
                <th
                  scope="col"
                  class="px-8 py-6 text-right text-xl font-bold text-gray-700"
                >
                  Aksi
                </th>
              </tr>
            </thead>
            <tbody class="bg-white divide-y divide-gray-200">
              <tr
                v-for="loan in loans"
                :key="loan.id"
                class="hover:bg-orange-50 transition"
              >
                <td class="px-8 py-6 whitespace-nowrap">
                  <div class="text-2xl font-bold text-gray-900">
                    {{ loan.id }}
                  </div>
                  <div class="text-xl text-gray-500 mt-1">{{ loan.date }}</div>
                </td>
                <td class="px-8 py-6 whitespace-nowrap">
                  <div class="text-2xl font-bold text-gray-900">
                    {{ loan.memberName }}
                  </div>
                </td>
                <td class="px-8 py-6 whitespace-nowrap">
                  <div class="text-2xl font-bold text-gray-800 tracking-wide">
                    {{ formatRupiah(loan.amount) }}
                  </div>
                </td>
                <td class="px-8 py-6 whitespace-nowrap">
                  <span
                    class="px-5 py-2 inline-flex text-xl font-bold rounded-full border"
                    :class="
                      loan.status === 'Lunas'
                        ? 'bg-green-100 text-green-800 border-green-300'
                        : 'bg-yellow-100 text-yellow-800 border-yellow-300'
                    "
                  >
                    {{ loan.status }}
                  </span>
                </td>
                <td class="px-8 py-6 whitespace-nowrap text-right">
                  <button
                    v-if="loan.status === 'Berjalan'"
                    @click="markAsPaid(loan.id)"
                    class="bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded-xl text-xl font-bold shadow-md transition flex items-center gap-2 justify-end float-right"
                  >
                    <CreditCard class="w-6 h-6" />
                    Bayar Lunas
                  </button>
                  <span v-else class="text-xl font-bold text-gray-400"
                    >Terselesaikan</span
                  >
                </td>
              </tr>
              <!-- Empty state jika tidak ada -->
              <tr v-if="loans.length === 0">
                <td
                  colspan="5"
                  class="px-8 py-12 text-center text-2xl text-gray-500 font-medium"
                >
                  Belum ada data pinjaman terdaftar.
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
    <!-- Modal Form (Sederhana) -->
    <div
      v-if="isModalOpen"
      class="fixed inset-0 z-50 flex items-center justify-center bg-gray-900/50 backdrop-blur-sm p-4"
    >
      <div
        class="bg-white rounded-3xl shadow-2xl p-10 w-full max-w-2xl border border-gray-100"
      >
        <h2
          class="text-3xl font-extrabold text-gray-900 mb-8 pb-4 border-b border-gray-200"
        >
          Form Pinjaman Baru
        </h2>

        <form @submit.prevent="submitNewLoan" class="space-y-6">
          <div>
            <label class="block text-2xl font-bold text-gray-700 mb-3"
              >Nama Anggota</label
            >
            <input
              v-model="newLoanForm.memberName"
              type="text"
              required
              class="w-full text-2xl p-5 border-2 border-gray-300 rounded-xl bg-gray-50 focus:border-orange-500 focus:ring-4 focus:ring-orange-100 outline-none transition"
              placeholder="Masukkan nama peminjam"
            />
          </div>
          <div>
            <label class="block text-2xl font-bold text-gray-700 mb-3"
              >Jumlah Pinjaman (Rp)</label
            >
            <input
              :value="newLoanAmountDisplay"
              @input="onAmountInput"
              type="text"
              required
              class="w-full text-2xl p-5 border-2 border-gray-300 rounded-xl bg-gray-50 focus:border-orange-500 focus:ring-4 focus:ring-orange-100 outline-none transition"
              placeholder="Contoh: 5.000.000"
            />
          </div>

          <div class="flex gap-4 pt-8">
            <button
              type="button"
              @click="isModalOpen = false"
              class="flex-1 py-5 bg-gray-200 text-gray-800 rounded-xl text-2xl font-bold hover:bg-gray-300 transition"
            >
              Batal
            </button>
            <button
              type="submit"
              class="flex-1 py-5 bg-blue-600 justify-center text-white rounded-xl text-2xl font-bold hover:bg-blue-700 transition shadow-lg flex items-center gap-3"
            >
              <CheckCircle class="w-8 h-8" />
              Simpan Pinjaman
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>
