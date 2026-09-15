import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { computed, reactive, ref } from "vue";
import { DEFAULT_LOCALE, translate } from "../i18n";

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
const refreshing = ref(false);
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
const FIRST_REPORTING_YEAR = 2025;
const currentYear = new Date().getFullYear();
const selectedYear = ref(currentYear);
const yearOptions = computed(() =>
  Array.from(
    { length: Math.max(1, currentYear - FIRST_REPORTING_YEAR + 1) },
    (_, index) => currentYear - index,
  ),
);
let toastId = 0;
let initializePromise: Promise<void> | null = null;
let refreshPromise: Promise<boolean> | null = null;

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function yearFrom(value: string) {
  const match = value.match(/(?:19|20)\d{2}/);
  return match ? Number(match[0]) : null;
}

const yearTransactions = computed(() =>
  transactions.filter(
    (transaction) => yearFrom(transaction.date) === selectedYear.value,
  ),
);
const yearLoans = computed(() =>
  loans.filter((loan) => {
    const year = yearFrom(loan.realizationDate);
    return (
      year === selectedYear.value ||
      (year === null && selectedYear.value === currentYear)
    );
  }),
);
const yearHasData = computed(
  () => yearTransactions.value.length > 0 || yearLoans.value.length > 0,
);
const yearMembers = computed(() => (yearHasData.value ? members : []));
const yearTotals = computed(() => ({
  cash: yearTransactions.value.reduce(
    (sum, transaction) =>
      sum +
      (transaction.direction === "Masuk"
        ? transaction.amount
        : -transaction.amount),
    0,
  ),
  savings: yearMembers.value.reduce((sum, member) => sum + member.savings, 0),
  loanPortfolio: yearLoans.value
    .filter(
      (loan) => loan.status === "Berjalan" || loan.status === "Perlu review",
    )
    .reduce((sum, loan) => sum + loan.balance, 0),
  members: yearMembers.value.filter((member) => member.status === "Aktif")
    .length,
}));

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
        translate("notifications.backendUnavailable"),
        translate("notifications.backendHint"),
        "warning",
      );
    } finally {
      loading.value = false;
      initializePromise = null;
    }
  })();
  return initializePromise;
}

async function refresh() {
  if (refreshPromise) return refreshPromise;
  refreshPromise = (async () => {
    refreshing.value = true;
    try {
      applySnapshot(await invoke<AppSnapshot>("get_app_snapshot"));
      return true;
    } catch (error) {
      notify(
        translate("notifications.refreshFailed"),
        errorMessage(error),
        "warning",
      );
      return false;
    } finally {
      refreshing.value = false;
      refreshPromise = null;
    }
  })();
  return refreshPromise;
}

async function importWorkbook() {
  const path = await open({
    title: translate("dialog.workbookPicker"),
    multiple: false,
    filters: [{ name: "Excel Macro Workbook", extensions: ["xlsm", "xlsx"] }],
  });
  if (!path || Array.isArray(path)) return false;
  try {
    applySnapshot(await invoke<AppSnapshot>("import_workbook", { path }));
    notify(
      translate("notifications.workbookImported"),
      translate("notifications.workbookImportedMessage"),
    );
    return true;
  } catch (error) {
    notify(
      translate("notifications.workbookImportFailed"),
      errorMessage(error),
      "warning",
    );
    return false;
  }
}

async function addMember(input: {
  name: string;
  memberNumber: string;
  joinedAt: string;
  principalSavings: number;
}) {
  try {
    applySnapshot(
      await invoke<AppSnapshot>("add_member", {
        input: { ...input, ...operationalTimestamp() },
      }),
    );
    notify(
      translate("notifications.memberAdded"),
      translate("notifications.memberAddedMessage", { name: input.name }),
    );
    return true;
  } catch (error) {
    notify(
      translate("notifications.memberSaveFailed"),
      errorMessage(error),
      "warning",
    );
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
      translate("notifications.loanDrafted"),
      translate("notifications.loanDraftedMessage"),
      "info",
    );
    return true;
  } catch (error) {
    notify(
      translate("notifications.loanSaveFailed"),
      errorMessage(error),
      "warning",
    );
    return false;
  }
}

function operationalTimestamp() {
  return {
    businessDate: "2026-09-13",
    displayDate: "13 Sep 2026",
    displayTime: new Date().toLocaleTimeString(DEFAULT_LOCALE, {
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
      translate("notifications.loanDisbursed"),
      translate("notifications.loanDisbursedMessage"),
    );
    return true;
  } catch (error) {
    notify(
      translate("notifications.loanDisbursementFailed"),
      errorMessage(error),
      "warning",
    );
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
      translate("notifications.movementPosted", { movement: input.movement }),
      translate("notifications.movementPostedMessage"),
    );
    return true;
  } catch (error) {
    notify(
      translate("notifications.movementFailed", { movement: input.movement }),
      errorMessage(error),
      "warning",
    );
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
      translate("notifications.paymentPosted"),
      translate("notifications.paymentPostedMessage", {
        amount: formatCurrency(
          input.principal + input.interest + input.wajib + input.voluntary,
        ),
      }),
    );
    return true;
  } catch (error) {
    notify(
      translate("notifications.paymentFailed"),
      errorMessage(error),
      "warning",
    );
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
    notify(
      translate("notifications.adminLoadFailed"),
      errorMessage(error),
      "warning",
    );
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
      translate("notifications.parametersSaved"),
      translate("notifications.parametersSavedMessage", {
        date: input.effectiveDate,
      }),
    );
    return true;
  } catch (error) {
    notify(
      translate("notifications.parametersSaveFailed"),
      errorMessage(error),
      "warning",
    );
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
      translate("notifications.reversalPosted"),
      translate("notifications.reversalPostedMessage", { id }),
      "warning",
    );
    return true;
  } catch (error) {
    notify(
      translate("notifications.reversalFailed"),
      errorMessage(error),
      "warning",
    );
    return false;
  }
}

export function formatCurrency(value: number, compact = false) {
  return new Intl.NumberFormat(DEFAULT_LOCALE, {
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
  refreshing,
  backendError,
  admin,
  toasts,
  selectedYear,
  yearOptions,
  yearTransactions,
  yearLoans,
  yearMembers,
  yearTotals,
  yearHasData,
  initialize,
  refresh,
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
