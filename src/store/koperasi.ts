import { invoke } from "@tauri-apps/api/core";
import { useQuery } from "@tanstack/vue-query";
import { computed, reactive, ref, watch } from "vue";
import { queryClient, queryKeys } from "../query-client";
import { DEFAULT_LOCALE, translate } from "../i18n";

export const TransactionStatus = {
  Posted: "Terposting",
  Reversed: "Dibalik",
} as const;
export type TransactionStatus =
  (typeof TransactionStatus)[keyof typeof TransactionStatus];

// Sections of PINJAMAN BULANAN.
export const LoanGroup = {
  Member: "Anggota PP BRI",
  NonMember: "Non Anggota PP BRI",
} as const;
export type LoanGroup = (typeof LoanGroup)[keyof typeof LoanGroup];

// Jenis Pinjaman column of PINJAMAN BULANAN.
export const LoanType = {
  Monthly: "Bulanan",
  Temporary: "Sementara",
} as const;
export type LoanType = (typeof LoanType)[keyof typeof LoanType];

export const TransactionDirection = {
  In: "Masuk",
  Out: "Keluar",
} as const;
export type TransactionDirection =
  (typeof TransactionDirection)[keyof typeof TransactionDirection];

export const SavingsAccountType = {
  Principal: "POKOK",
  Mandatory: "WAJIB",
  Voluntary: "MANASUKA",
} as const;
export type SavingsAccountType =
  (typeof SavingsAccountType)[keyof typeof SavingsAccountType];

export const SavingsMovement = {
  Deposit: "Setoran",
  Withdrawal: "Penarikan",
} as const;
export type SavingsMovement =
  (typeof SavingsMovement)[keyof typeof SavingsMovement];

// KAS = recorded in the daily cash ledger; NON_KAS = savings/loans only.
export const Channel = {
  Cash: "KAS",
  NonCash: "NON_KAS",
} as const;
export type Channel = (typeof Channel)[keyof typeof Channel];

// Spelled as in PINJAMAN BULANAN.
export const InterestType = {
  Declining: "Menurun",
  Flat: "Plat",
} as const;
export type InterestType = (typeof InterestType)[keyof typeof InterestType];

// Category columns of Buku Kas TAHUN 2026, in sheet order.
export const cashCategories = [
  { value: "KAS_BUKU_TABUNGAN", label: "KAS BUKU TABUNGAN" },
  { value: "BIAYA_PENGURUS", label: "Biaya Pengurus" },
  { value: "BIAYA_ATK", label: "Biaya ATK" },
  { value: "TRANSFORTASI", label: "Transfortasi" },
  { value: "BIAYA_BUNGA", label: "Biaya Bunga" },
  { value: "HUMAS", label: "Humas" },
  { value: "PEMELIHARAAN_AT", label: "Pemeliharaan AT" },
  { value: "BIAYA_RAT", label: "Biaya RAT" },
  { value: "SEWA_KANTOR", label: "Sewa Kantor" },
  { value: "DANSOS", label: "Dansos" },
  { value: "LAINNYA", label: "Lainnya" },
  { value: "DEKOPINDA", label: "Dekopinda" },
  { value: "PARCEL", label: "Parcel" },
] as const;
export type CashCategory = (typeof cashCategories)[number]["value"];

export interface Company {
  id: string;
  name: string;
}

export interface Member {
  id: string;
  name: string;
  joinedAt: string;
  principalSavings: number;
}

export interface SavingsBalance {
  memberId: string;
  accountType: SavingsAccountType;
  balance: number;
}

export interface Loan {
  id: string;
  loanGroup: LoanGroup;
  /** Empty for Non Anggota PP BRI. */
  memberId: string;
  memberName: string;
  plafond: number;
  balance: number;
  rate: number;
  tenor: number;
  interestType: InterestType;
  /** Empty where the sheet leaves Jenis Pinjaman blank. */
  loanType: LoanType | null;
  realizationDate: string;
  dueDate: string;
  guarantee: string;
}

export interface SavingsMonth {
  memberId: string;
  period: string;
  transactionDate: string;
  principalOpening: number;
  mandatoryOpening: number;
  voluntaryOpening: number;
  principalIn: number;
  principalOut: number;
  mandatoryIn: number;
  mandatoryOut: number;
  voluntaryIn: number;
  voluntaryOut: number;
  shu: number;
  principalClosing: number;
  mandatoryClosing: number;
  voluntaryClosing: number;
}

export interface LoanMonth {
  loanId: string;
  period: string;
  transactionDate: string;
  openingBalance: number;
  disbursed: number;
  principalPaid: number;
  interestPaid: number;
  provision: number;
  scheduledPrincipal: number;
  scheduledInterest: number;
  arrearsPrincipal: number;
  arrearsInterest: number;
  closingBalance: number;
}

export interface MonthlyLedger {
  periods: string[];
  savings: SavingsMonth[];
  loans: LoanMonth[];
}

export interface Transaction {
  id: string;
  date: string;
  time: string;
  memberName: string;
  channel: Channel;
  description: string;
  reference: string;
  direction: TransactionDirection;
  amount: number;
  status: TransactionStatus;
  /** The row that cancels another row (Dibalik). */
  isReversal: boolean;
  components: { label: string; amount: number }[];
  actor: string;
}

export interface CashBookRow extends Transaction {
  businessDate: string;
  balance: number;
}

export interface CashBook {
  channel: Channel;
  year: number;
  openingBalance: number;
  totalIn: number;
  totalOut: number;
  closingBalance: number;
  rows: CashBookRow[];
}

/** Optional date and channel for each cash ledger posting. */
export interface PostingOptions {
  channel?: Channel;
  businessDate?: string;
}

export interface CashEntryInput {
  direction: TransactionDirection;
  /** Empty when no Buku Kas category column applies. */
  category: CashCategory | "";
  description: string;
  amount: number;
  reference: string;
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

interface AddMemberInput {
  name: string;
  joinedAt: string;
  openingSavings: {
    accountType: SavingsAccountType;
    amount: number;
  }[];
}

interface CreateLoanInput {
  loanGroup: LoanGroup;
  memberId: string;
  /** Borrower name for Non Anggota PP BRI. */
  borrowerName: string;
  plafond: number;
  tenor: number;
  interestType: InterestType;
  loanType: LoanType;
}

type ChannelInput = { channel: Channel };

interface SavingsTransactionInput {
  memberId: string;
  accountType: SavingsAccountType;
  movement: SavingsMovement;
  amount: number;
  reference: string;
}

interface PaymentInput {
  memberId: string;
  principal: number;
  interest: number;
  wajib: number;
  voluntary: number;
  reference: string;
}

type OperationalTimestamp = ReturnType<typeof operationalTimestamp>;

interface AppSnapshot {
  members: Member[];
  savingsBalances: SavingsBalance[];
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
const financialParameters = reactive<FinancialParameters>({
  principalSavings: 0,
  mandatorySavings: 0,
  provisionRate: 0,
  annualRate: 0,
  effectiveDate: "",
});
const toasts = ref<ToastMessage[]>([]);
const snapshotVersion = ref(0);
const companies = ref<Company[]>([]);
const configuredCompanyId = import.meta.env.VITE_COMPANY_ID?.trim() || "";
const selectedCompanyId = ref(configuredCompanyId || "default");

const command = {
  listCompanies: "list_companies",
  getAppSnapshot: "get_app_snapshot",
  getMonthlyLedger: "get_monthly_ledger",
  getCashBook: "get_cash_book",
  postCashEntry: "post_cash_entry",
  addMember: "add_member",
  previewLoan: "preview_loan",
  createLoan: "create_loan",
  postSavingsTransaction: "post_savings_transaction",
  postPayment: "post_payment",
  reverseTransaction: "reverse_transaction",
  getFinancialParameters: "get_financial_parameters",
} as const;

const backend = {
  listCompanies: () => invoke<Company[]>(command.listCompanies),
  getAppSnapshot: (companyId = selectedCompanyId.value) =>
    invoke<AppSnapshot>(command.getAppSnapshot, {
      companyId,
    }),
  getMonthlyLedger: (year: number, companyId = selectedCompanyId.value) =>
    invoke<MonthlyLedger>(command.getMonthlyLedger, {
      companyId,
      year,
    }),
  getCashBook: (
    year: number,
    channel: Channel,
    companyId = selectedCompanyId.value,
  ) =>
    invoke<CashBook>(command.getCashBook, {
      companyId,
      year,
      channel,
    }),
  postCashEntry: (
    input: CashEntryInput & ChannelInput & OperationalTimestamp,
  ) =>
    invoke<AppSnapshot>(command.postCashEntry, {
      input,
      companyId: selectedCompanyId.value,
    }),
  addMember: (input: AddMemberInput & ChannelInput & OperationalTimestamp) =>
    invoke<AppSnapshot>(command.addMember, {
      input,
      companyId: selectedCompanyId.value,
    }),
  previewLoan: (input: CreateLoanInput) =>
    invoke<LoanPreview>(command.previewLoan, {
      input,
      companyId: selectedCompanyId.value,
    }),
  createLoan: (
    input: CreateLoanInput & {
      disbursement: ChannelInput & OperationalTimestamp;
    },
  ) =>
    invoke<AppSnapshot>(command.createLoan, {
      input,
      companyId: selectedCompanyId.value,
    }),
  postSavingsTransaction: (
    input: SavingsTransactionInput & ChannelInput & OperationalTimestamp,
  ) =>
    invoke<AppSnapshot>(command.postSavingsTransaction, {
      input,
      companyId: selectedCompanyId.value,
    }),
  postPayment: (input: PaymentInput & ChannelInput & OperationalTimestamp) =>
    invoke<AppSnapshot>(command.postPayment, {
      input,
      companyId: selectedCompanyId.value,
    }),
  reverseTransaction: (input: { id: string } & OperationalTimestamp) =>
    invoke<AppSnapshot>(command.reverseTransaction, {
      input,
      companyId: selectedCompanyId.value,
    }),
  getFinancialParameters: (companyId = selectedCompanyId.value) =>
    invoke<FinancialParameters>(command.getFinancialParameters, {
      companyId,
    }),
};

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

function applySnapshot(
  snapshot: AppSnapshot,
  companyId = selectedCompanyId.value,
) {
  if (companyId !== selectedCompanyId.value) return;
  members.splice(0, members.length, ...snapshot.members);
  loans.splice(0, loans.length, ...snapshot.loans);
  transactions.splice(0, transactions.length, ...snapshot.transactions);
  totals.cash.value = snapshot.totals.cash;
  totals.savings.value = snapshot.totals.savings;
  totals.loanPortfolio.value = snapshot.totals.loanPortfolio;
  totals.members.value = snapshot.totals.members;
  backendError.value = null;
  snapshotVersion.value += 1;
}

function applyMutationSnapshot(snapshot: AppSnapshot, companyId: string) {
  void queryClient.cancelQueries({
    queryKey: queryKeys.snapshot(companyId),
    exact: true,
  });
  queryClient.setQueryData(queryKeys.snapshot(companyId), snapshot);
  void queryClient.invalidateQueries({
    queryKey: queryKeys.company(companyId),
    predicate: (query) =>
      query.queryKey[2] === "monthly-ledger" ||
      query.queryKey[2] === "cash-book",
  });
  applySnapshot(snapshot, companyId);
}

function fetchSnapshot(companyId: string) {
  return queryClient.fetchQuery({
    queryKey: queryKeys.snapshot(companyId),
    queryFn: () => backend.getAppSnapshot(companyId),
  });
}

async function initialize() {
  if (initializePromise) return initializePromise;
  initializePromise = (async () => {
    loading.value = true;
    try {
      companies.value = await queryClient.fetchQuery({
        queryKey: queryKeys.companies,
        queryFn: backend.listCompanies,
      });
      if (
        configuredCompanyId &&
        !companies.value.some((company) => company.id === configuredCompanyId)
      ) {
        throw new Error(
          `VITE_COMPANY_ID "${configuredCompanyId}" tidak ditemukan dalam daftar perusahaan.`,
        );
      }
      if (
        !companies.value.some(
          (company) => company.id === selectedCompanyId.value,
        )
      ) {
        selectedCompanyId.value = companies.value[0]?.id ?? "default";
      }
      const companyId = selectedCompanyId.value;
      applySnapshot(await fetchSnapshot(companyId), companyId);
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
      const companyId = selectedCompanyId.value;
      await queryClient.invalidateQueries({
        queryKey: queryKeys.company(companyId),
        refetchType: "none",
      });
      applySnapshot(await fetchSnapshot(companyId), companyId);
      await queryClient.refetchQueries({
        queryKey: queryKeys.company(companyId),
        type: "active",
      });
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

async function selectCompany(companyId: string) {
  if (configuredCompanyId && companyId !== configuredCompanyId) return false;
  if (companyId === selectedCompanyId.value) return true;
  selectedCompanyId.value = companyId;
  const cached = queryClient.getQueryData<AppSnapshot>(
    queryKeys.snapshot(companyId),
  );
  loading.value = !cached;
  backendError.value = null;
  if (cached) applySnapshot(cached, companyId);
  try {
    applySnapshot(await fetchSnapshot(companyId), companyId);
    if (companyId !== selectedCompanyId.value) return false;
    await loadFinancialParameters();
    return true;
  } catch (error) {
    if (companyId !== selectedCompanyId.value) return false;
    if (!cached) backendError.value = errorMessage(error);
    notify(
      translate("notifications.refreshFailed"),
      errorMessage(error),
      "warning",
    );
    return false;
  } finally {
    if (companyId === selectedCompanyId.value) loading.value = false;
  }
}

async function addMember(input: AddMemberInput, options: PostingOptions = {}) {
  const companyId = selectedCompanyId.value;
  try {
    applyMutationSnapshot(
      await backend.addMember({
        ...input,
        channel: options.channel ?? Channel.Cash,
        ...operationalTimestamp(options.businessDate),
      }),
      companyId,
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

async function previewLoan(input: CreateLoanInput) {
  return backend.previewLoan(input);
}

/** Loans are recorded when they are realised (disbursed through the cash ledger). */
async function createLoan(
  input: CreateLoanInput,
  disburse: PostingOptions = {},
) {
  const companyId = selectedCompanyId.value;
  try {
    applyMutationSnapshot(
      await backend.createLoan({
        ...input,
        disbursement: {
          channel: disburse.channel ?? Channel.Cash,
          ...operationalTimestamp(disburse.businessDate),
        },
      }),
      companyId,
    );
    notify(
      translate("notifications.loanDisbursed"),
      translate("notifications.loanDisbursedMessage"),
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

/** Today's date (local time zone) in YYYY-MM-DD format. */
export function todayIso() {
  const now = new Date();
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

function operationalTimestamp(businessDate = todayIso()) {
  const [year, month, day] = businessDate.split("-").map(Number);
  return {
    businessDate,
    displayDate: new Date(year, month - 1, day).toLocaleDateString(
      DEFAULT_LOCALE,
      { day: "2-digit", month: "short", year: "numeric" },
    ),
    displayTime: new Date().toLocaleTimeString(DEFAULT_LOCALE, {
      hour: "2-digit",
      minute: "2-digit",
    }),
  };
}

async function postSavingsTransaction(
  input: SavingsTransactionInput,
  options: PostingOptions = {},
) {
  const companyId = selectedCompanyId.value;
  try {
    applyMutationSnapshot(
      await backend.postSavingsTransaction({
        ...input,
        channel: options.channel ?? Channel.Cash,
        ...operationalTimestamp(options.businessDate),
      }),
      companyId,
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

async function postPayment(input: PaymentInput, options: PostingOptions = {}) {
  const backendInput = {
    ...input,
    channel: options.channel ?? Channel.Cash,
    ...operationalTimestamp(options.businessDate),
  };
  const companyId = selectedCompanyId.value;
  try {
    applyMutationSnapshot(await backend.postPayment(backendInput), companyId);
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

async function loadFinancialParameters() {
  try {
    const companyId = selectedCompanyId.value;
    const parameters = await queryClient.fetchQuery({
      queryKey: queryKeys.parameters(companyId),
      queryFn: () => backend.getFinancialParameters(companyId),
    });
    if (companyId === selectedCompanyId.value) {
      Object.assign(financialParameters, parameters);
    }
    return true;
  } catch (error) {
    notify(
      translate("notifications.parametersLoadFailed"),
      errorMessage(error),
      "warning",
    );
    return false;
  }
}

async function postCashEntry(
  input: CashEntryInput,
  options: PostingOptions = {},
) {
  const companyId = selectedCompanyId.value;
  try {
    applyMutationSnapshot(
      await backend.postCashEntry({
        ...input,
        channel: options.channel ?? Channel.Cash,
        ...operationalTimestamp(options.businessDate),
      }),
      companyId,
    );
    notify(
      translate("notifications.paymentPosted"),
      `${input.description} · ${formatCurrency(input.amount)}`,
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

async function getCashBook(year: number, channel: Channel) {
  const companyId = selectedCompanyId.value;
  return queryClient.fetchQuery({
    queryKey: queryKeys.cashBook(companyId, year, channel),
    queryFn: () => backend.getCashBook(year, channel, companyId),
  });
}

async function reverseTransaction(id: string) {
  const companyId = selectedCompanyId.value;
  try {
    applyMutationSnapshot(
      await backend.reverseTransaction({ id, ...operationalTimestamp() }),
      companyId,
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

const emptyMonthlyLedger: MonthlyLedger = {
  periods: [],
  savings: [],
  loans: [],
};

/** Shared reports are cached separately for each company and reporting year. */
export function useMonthlyLedger() {
  const query = useQuery(
    computed(() => {
      const companyId = selectedCompanyId.value;
      const year = selectedYear.value;
      return {
        queryKey: queryKeys.monthlyLedger(companyId, year),
        queryFn: () => backend.getMonthlyLedger(year, companyId),
      };
    }),
  );
  watch(query.error, (error) => {
    if (error)
      notify(
        translate("notifications.refreshFailed"),
        errorMessage(error),
        "warning",
      );
  });
  return {
    monthlyLedger: computed(() => query.data.value ?? emptyMonthlyLedger),
    monthlyLedgerLoading: query.isPending,
  };
}

export function useCashBook(channel: Channel) {
  const query = useQuery(
    computed(() => {
      const companyId = selectedCompanyId.value;
      const year = selectedYear.value;
      return {
        queryKey: queryKeys.cashBook(companyId, year, channel),
        queryFn: () => backend.getCashBook(year, channel, companyId),
      };
    }),
  );
  watch(query.error, (error) => {
    if (error) notify("Buku kas gagal dimuat", errorMessage(error), "warning");
  });
  return { cashBook: query.data, loadingBook: query.isPending };
}

const monthNames = [
  "Januari",
  "Februari",
  "Maret",
  "April",
  "Mei",
  "Juni",
  "Juli",
  "Agustus",
  "September",
  "Oktober",
  "November",
  "Desember",
];

/** "2026-09" -> "September 2026". */
export function formatPeriod(period: string) {
  const [year, month] = period.split("-").map(Number);
  return `${monthNames[month - 1] ?? period} ${year}`;
}

/** Month before the period, e.g. "2026-01" -> "December 2025" (localized). */
export function formatPreviousPeriod(period: string) {
  const [year, month] = period.split("-").map(Number);
  return month === 1
    ? `${monthNames[11]} ${year - 1}`
    : `${monthNames[month - 2]} ${year}`;
}

/** "2026-09-02" -> "02-09-2026"; empty input becomes "". */
export function formatSheetDate(value: string) {
  const match = value.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  return match ? `${match[3]}-${match[2]}-${match[1]}` : value;
}

/** Spreadsheet-style numbers: periods for thousands, parentheses for negatives, "-" for zero. */
const sheetNumberFormatter = new Intl.NumberFormat(DEFAULT_LOCALE, {
  maximumFractionDigits: 0,
});

export function formatSheetNumber(value: number) {
  if (!value) return "-";
  const formatted = sheetNumberFormatter.format(Math.abs(value));
  return value < 0 ? `(${formatted})` : formatted;
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
  financialParameters,
  toasts,
  companies,
  configuredCompanyId,
  selectedCompanyId,
  selectedYear,
  yearOptions,
  initialize,
  selectCompany,
  refresh,
  notify,
  addMember,
  previewLoan,
  createLoan,
  postSavingsTransaction,
  postPayment,
  reverseTransaction,
  postCashEntry,
  getCashBook,
  snapshotVersion,
  loadFinancialParameters,
});
