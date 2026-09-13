import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { computed, reactive, ref } from "vue";

export type TransactionStatus = "Terposting" | "Draf" | "Dibalik";
export type LoanStatus = "Draf" | "Berjalan" | "Perlu review" | "Lunas";

export interface Member {
  id: string;
  name: string;
  memberNumber: string;
  joinedAt: string;
  status: "Aktif" | "Nonaktif";
  savings: number;
  principalSavings: number;
  mandatorySavings: number;
  voluntarySavings: number;
  loanBalance: number;
}

export interface Loan {
  id: string;
  memberId: string;
  memberName: string;
  plafond: number;
  balance: number;
  rate: number;
  tenor: number;
  interestType: "Menurun" | "Flat";
  realizationDate: string;
  dueDate: string;
  status: LoanStatus;
}

export interface Transaction {
  id: string;
  date: string;
  time: string;
  memberName: string;
  description: string;
  reference: string;
  direction: "Masuk" | "Keluar";
  amount: number;
  status: TransactionStatus;
  components: { label: string; amount: number }[];
  actor: string;
}

export interface LoanPreview {
  principalInstallment: number;
  firstInterest: number;
  provision: number;
  firstTotal: number;
  annualRate: number;
}

export interface FinancialParameters {
  principalSavings: number;
  mandatorySavings: number;
  provisionRate: number;
  annualRate: number;
  effectiveDate: string;
}

export interface AuditEvent {
  id: number;
  entityType: string;
  entityId: string;
  action: string;
  actor: string;
  createdAt: string;
}

interface AdminState {
  parameters: FinancialParameters;
  auditEvents: AuditEvent[];
}

interface AppSnapshot {
  members: Member[];
  loans: Loan[];
  transactions: Transaction[];
  totals: {
    cash: number;
    savings: number;
    loanPortfolio: number;
    members: number;
  };
}

export interface ToastMessage {
  id: number;
  title: string;
  message: string;
  tone: "success" | "info" | "warning";
}

const members = reactive<Member[]>([]);
const loans = reactive<Loan[]>([]);
const transactions = reactive<Transaction[]>([]);
const totals = {
  cash: ref(0),
  savings: ref(0),
  loanPortfolio: ref(0),
  members: ref(0),
};
const loading = ref(true);
const backendError = ref<string | null>(null);
const admin = reactive<AdminState>({
  parameters: {
    principalSavings: 0,
    mandatorySavings: 0,
    provisionRate: 0,
    annualRate: 0,
    effectiveDate: "",
  },
  auditEvents: [],
});
const toasts = ref<ToastMessage[]>([]);
let toastId = 0;
let initializePromise: Promise<void> | null = null;

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function notify(
  title: string,
  message: string,
  tone: ToastMessage["tone"] = "success",
) {
  const id = ++toastId;
  toasts.value.push({ id, title, message, tone });
  window.setTimeout(() => {
    toasts.value = toasts.value.filter((item) => item.id !== id);
  }, 4200);
}

function applySnapshot(snapshot: AppSnapshot) {
  members.splice(0, members.length, ...snapshot.members);
  loans.splice(0, loans.length, ...snapshot.loans);
  transactions.splice(0, transactions.length, ...snapshot.transactions);
  totals.cash.value = snapshot.totals.cash;
  totals.savings.value = snapshot.totals.savings;
  totals.loanPortfolio.value = snapshot.totals.loanPortfolio;
  totals.members.value = snapshot.totals.members;
  backendError.value = null;
}

async function initialize() {
  if (initializePromise) return initializePromise;
  initializePromise = (async () => {
    loading.value = true;
    try {
      applySnapshot(await invoke<AppSnapshot>("get_app_snapshot"));
    } catch (error) {
      backendError.value = errorMessage(error);
      notify(
        "Backend tidak tersedia",
        "Jalankan aplikasi melalui `pnpm tauri dev`, bukan server web biasa.",
        "warning",
      );
    } finally {
      loading.value = false;
      initializePromise = null;
    }
  })();
  return initializePromise;
}

async function importWorkbook() {
  const path = await open({
    title: "Pilih workbook Excel (.xlsm)",
    multiple: false,
    filters: [{ name: "Excel Macro Workbook", extensions: ["xlsm", "xlsx"] }],
  });
  if (!path || Array.isArray(path)) return false;
  try {
    applySnapshot(await invoke<AppSnapshot>("import_workbook", { path }));
    notify(
      "Workbook berhasil diimpor",
      "Anggota, pinjaman, simpanan, dan buku kas telah dimuat ke database lokal.",
    );
    return true;
  } catch (error) {
    notify("Impor workbook gagal", errorMessage(error), "warning");
    return false;
  }
}

async function addMember(input: {
  name: string;
  memberNumber: string;
  joinedAt: string;
}) {
  try {
    applySnapshot(await invoke<AppSnapshot>("add_member", { input }));
    notify(
      "Anggota berhasil ditambahkan",
      `${input.name} dan tiga rekening simpanannya tersimpan di SQLite.`,
    );
    return true;
  } catch (error) {
    notify("Anggota gagal disimpan", errorMessage(error), "warning");
    return false;
  }
}

async function previewLoan(input: {
  memberId: string;
  plafond: number;
  tenor: number;
  interestType: "Menurun" | "Flat";
}) {
  return invoke<LoanPreview>("preview_loan", { input });
}

async function createLoan(input: {
  memberId: string;
  plafond: number;
  tenor: number;
  interestType: "Menurun" | "Flat";
}) {
  try {
    applySnapshot(await invoke<AppSnapshot>("create_loan", { input }));
    notify(
      "Pinjaman disimpan sebagai draf",
      "Kontrak menunggu persetujuan sebelum pencairan.",
      "info",
    );
    return true;
  } catch (error) {
    notify("Pinjaman gagal disimpan", errorMessage(error), "warning");
    return false;
  }
}

function operationalTimestamp() {
  return {
    businessDate: "2026-09-13",
    displayDate: "13 Sep 2026",
    displayTime: new Date().toLocaleTimeString("id-ID", {
      hour: "2-digit",
      minute: "2-digit",
    }),
  };
}

async function disburseLoan(loanId: string) {
  try {
    applySnapshot(
      await invoke<AppSnapshot>("disburse_loan", {
        input: { loanId, ...operationalTimestamp() },
      }),
    );
    notify(
      "Pinjaman berhasil dicairkan",
      "Kas keluar, piutang pokok, provisi, dan audit trail telah diposting atomik.",
    );
    return true;
  } catch (error) {
    notify("Pencairan pinjaman gagal", errorMessage(error), "warning");
    return false;
  }
}

async function postSavingsTransaction(input: {
  memberId: string;
  accountType: "POKOK" | "WAJIB" | "MANASUKA";
  movement: "Setoran" | "Penarikan";
  amount: number;
  reference: string;
}) {
  try {
    applySnapshot(
      await invoke<AppSnapshot>("post_savings_transaction", {
        input: { ...input, ...operationalTimestamp() },
      }),
    );
    notify(
      `${input.movement} berhasil diposting`,
      "Saldo simpanan, buku kas, dan audit trail telah diperbarui.",
    );
    return true;
  } catch (error) {
    notify(`${input.movement} gagal`, errorMessage(error), "warning");
    return false;
  }
}

async function postPayment(input: {
  memberId: string;
  principal: number;
  interest: number;
  wajib: number;
  voluntary: number;
  reference: string;
}) {
  const backendInput = {
    ...input,
    ...operationalTimestamp(),
  };
  try {
    applySnapshot(
      await invoke<AppSnapshot>("post_payment", { input: backendInput }),
    );
    notify(
      "Pembayaran berhasil diposting",
      `${formatCurrency(input.principal + input.interest + input.wajib + input.voluntary)} masuk melalui transaksi database atomik.`,
    );
    return true;
  } catch (error) {
    notify("Pembayaran gagal diposting", errorMessage(error), "warning");
    return false;
  }
}

async function loadAdminState() {
  try {
    const state = await invoke<AdminState>("get_admin_state");
    Object.assign(admin.parameters, state.parameters);
    admin.auditEvents.splice(0, admin.auditEvents.length, ...state.auditEvents);
    return true;
  } catch (error) {
    notify("Administrasi gagal dimuat", errorMessage(error), "warning");
    return false;
  }
}

async function saveFinancialParameters(input: FinancialParameters) {
  try {
    const state = await invoke<AdminState>("save_financial_parameters", {
      input,
    });
    Object.assign(admin.parameters, state.parameters);
    admin.auditEvents.splice(0, admin.auditEvents.length, ...state.auditEvents);
    notify(
      "Versi parameter tersimpan",
      `Nilai baru berlaku mulai ${input.effectiveDate} dan tercatat di audit trail.`,
    );
    return true;
  } catch (error) {
    notify("Parameter gagal disimpan", errorMessage(error), "warning");
    return false;
  }
}

async function reverseTransaction(id: string) {
  try {
    applySnapshot(
      await invoke<AppSnapshot>("reverse_transaction", {
        input: { id, ...operationalTimestamp() },
      }),
    );
    notify(
      "Reversal berhasil diposting",
      `${id} dibalik melalui transaksi baru tanpa menghapus histori asal.`,
      "warning",
    );
    return true;
  } catch (error) {
    notify("Reversal gagal", errorMessage(error), "warning");
    return false;
  }
}

export function formatCurrency(value: number, compact = false) {
  return new Intl.NumberFormat("id-ID", {
    style: "currency",
    currency: "IDR",
    maximumFractionDigits: 0,
    notation: compact ? "compact" : "standard",
  }).format(value);
}

export const useKoperasiStore = () => ({
  members,
  loans,
  transactions,
  totals,
  loading,
  backendError,
  admin,
  toasts,
  initialize,
  notify,
  importWorkbook,
  addMember,
  previewLoan,
  createLoan,
  disburseLoan,
  postSavingsTransaction,
  postPayment,
  reverseTransaction,
  loadAdminState,
  saveFinancialParameters,
  activeLoans: computed(() =>
    loans.filter((loan) => loan.status === "Berjalan"),
  ),
});
