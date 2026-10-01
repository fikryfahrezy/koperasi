import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { computed, effectScope, reactive, ref, watch } from "vue";
import { DEFAULT_LOCALE, translate } from "../i18n";

export const TransactionStatus = {
  Posted: "Terposting",
  Draft: "Draf",
  Reversed: "Dibalik",
} as const;
export type TransactionStatus =
  (typeof TransactionStatus)[keyof typeof TransactionStatus];

export const LoanStatus = {
  Draft: "Draf",
  Active: "Berjalan",
  NeedsReview: "Perlu review",
  PaidOff: "Lunas",
} as const;
export type LoanStatus = (typeof LoanStatus)[keyof typeof LoanStatus];

export const MemberStatus = {
  Active: "Aktif",
  Inactive: "Nonaktif",
} as const;
export type MemberStatus = (typeof MemberStatus)[keyof typeof MemberStatus];

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

// Dari/ke mana uang bergerak. Buku Kas Harian = channel Kas.
export const Channel = {
  Cash: "KAS",
  Bank: "BANK",
  Deduction: "POTONGAN",
  NonCash: "NON_KAS",
} as const;
export type Channel = (typeof Channel)[keyof typeof Channel];
export type MoneyChannel = Exclude<Channel, typeof Channel.NonCash>;

export const channelLabels: Record<Channel, string> = {
  KAS: "Kas",
  BANK: "Bank",
  POTONGAN: "Potongan pensiun",
  NON_KAS: "Saldo awal",
};

export const InterestType = {
  Declining: "Menurun",
  Flat: "Flat",
} as const;
export type InterestType = (typeof InterestType)[keyof typeof InterestType];

export interface Company {
  id: string;
  name: string;
}

export interface Member {
  id: string;
  name: string;
  joinedAt: string;
  status: MemberStatus;
  principalSavings: number;
}

export interface SavingsBalance {
  memberId: string;
  accountType: SavingsAccountType;
  balance: number;
}

export interface Loan {
  id: string;
  memberId: string;
  memberName: string;
  plafond: number;
  balance: number;
  rate: number;
  tenor: number;
  interestType: InterestType;
  realizationDate: string;
  dueDate: string;
  guarantee: string;
  status: LoanStatus;
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
  transactionType: string;
  channel: Channel;
  description: string;
  reference: string;
  direction: TransactionDirection;
  amount: number;
  status: TransactionStatus;
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

/** Tanggal dan channel opsional untuk setiap posting dari Buku Kas. */
export interface PostingOptions {
  channel?: MoneyChannel;
  businessDate?: string;
}

export interface CashEntryInput {
  direction: TransactionDirection;
  category: string;
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
  memberId: string;
  plafond: number;
  tenor: number;
  interestType: InterestType;
}

type ChannelInput = { channel: MoneyChannel };

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
const savingsBalances = reactive<SavingsBalance[]>([]);
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
const snapshotVersion = ref(0);
const monthlyLedger = reactive<MonthlyLedger>({
  periods: [],
  savings: [],
  loans: [],
});
const monthlyLedgerLoading = ref(false);
const companies = ref<Company[]>([]);
const selectedCompanyId = ref("default");

const command = {
  listCompanies: "list_companies",
  getAppSnapshot: "get_app_snapshot",
  getMonthlyLedger: "get_monthly_ledger",
  getCashBook: "get_cash_book",
  postCashEntry: "post_cash_entry",
  importWorkbook: "import_workbook",
  addMember: "add_member",
  previewLoan: "preview_loan",
  createLoan: "create_loan",
  disburseLoan: "disburse_loan",
  postSavingsTransaction: "post_savings_transaction",
  postPayment: "post_payment",
  reverseTransaction: "reverse_transaction",
  getAdminState: "get_admin_state",
  saveFinancialParameters: "save_financial_parameters",
} as const;

const backend = {
  listCompanies: () => invoke<Company[]>(command.listCompanies),
  getAppSnapshot: () =>
    invoke<AppSnapshot>(command.getAppSnapshot, {
      companyId: selectedCompanyId.value,
    }),
  getMonthlyLedger: (year: number) =>
    invoke<MonthlyLedger>(command.getMonthlyLedger, {
      companyId: selectedCompanyId.value,
      year,
    }),
  getCashBook: (year: number, channel: Channel) =>
    invoke<CashBook>(command.getCashBook, {
      companyId: selectedCompanyId.value,
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
  importWorkbook: (path: string) =>
    invoke<AppSnapshot>(command.importWorkbook, {
      path,
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
      disbursement?: ChannelInput & OperationalTimestamp;
    },
  ) =>
    invoke<AppSnapshot>(command.createLoan, {
      input,
      companyId: selectedCompanyId.value,
    }),
  disburseLoan: (
    input: { loanId: string } & ChannelInput & OperationalTimestamp,
  ) =>
    invoke<AppSnapshot>(command.disburseLoan, {
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
  getAdminState: () =>
    invoke<AdminState>(command.getAdminState, {
      companyId: selectedCompanyId.value,
    }),
  saveFinancialParameters: (input: FinancialParameters) =>
    invoke<AdminState>(command.saveFinancialParameters, {
      input,
      companyId: selectedCompanyId.value,
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

function yearFrom(value: string) {
  const match = value.match(/(?:19|20)\d{2}/);
  return match ? Number(match[0]) : null;
}

const yearTransactions = computed(() =>
  transactions.filter(
    (transaction) => yearFrom(transaction.date) === selectedYear.value,
  ),
);
// Pinjaman yang direalisasi pada tahun terpilih, ditambah pinjaman lama yang
// masih berjalan, agar outstanding sama dengan saldo di workbook.
const yearLoans = computed(() =>
  loans.filter((loan) => {
    const year = yearFrom(loan.realizationDate);
    if (year === null) return selectedYear.value === currentYear;
    return (
      year === selectedYear.value ||
      (year < selectedYear.value && loan.status !== LoanStatus.PaidOff)
    );
  }),
);
const yearHasData = computed(
  () => yearTransactions.value.length > 0 || yearLoans.value.length > 0,
);
const yearMembers = computed(() => (yearHasData.value ? members : []));
const yearTotals = computed(() => ({
  // Saldo kas tunai dari buku besar (seluruh transaksi channel Kas).
  cash: totals.cash.value,
  savings: yearMembers.value.reduce(
    (sum, member) => sum + memberSavings(member, savingsBalances),
    0,
  ),
  loanPortfolio: yearLoans.value
    .filter(
      (loan) =>
        loan.status === LoanStatus.Active ||
        loan.status === LoanStatus.NeedsReview,
    )
    .reduce((sum, loan) => sum + loan.balance, 0),
  members: yearMembers.value.filter(
    (member) => member.status === MemberStatus.Active,
  ).length,
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
  savingsBalances.splice(
    0,
    savingsBalances.length,
    ...snapshot.savingsBalances,
  );
  loans.splice(0, loans.length, ...snapshot.loans);
  transactions.splice(0, transactions.length, ...snapshot.transactions);
  totals.cash.value = snapshot.totals.cash;
  totals.savings.value = snapshot.totals.savings;
  totals.loanPortfolio.value = snapshot.totals.loanPortfolio;
  totals.members.value = snapshot.totals.members;
  backendError.value = null;
  snapshotVersion.value += 1;
}

async function initialize() {
  if (initializePromise) return initializePromise;
  initializePromise = (async () => {
    loading.value = true;
    try {
      companies.value = await backend.listCompanies();
      if (
        !companies.value.some(
          (company) => company.id === selectedCompanyId.value,
        )
      ) {
        selectedCompanyId.value = companies.value[0]?.id ?? "default";
      }
      applySnapshot(await backend.getAppSnapshot());
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
      applySnapshot(await backend.getAppSnapshot());
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
  if (companyId === selectedCompanyId.value) return true;
  selectedCompanyId.value = companyId;
  loading.value = true;
  try {
    applySnapshot(await backend.getAppSnapshot());
    await loadAdminState();
    return true;
  } catch (error) {
    backendError.value = errorMessage(error);
    notify(
      translate("notifications.refreshFailed"),
      errorMessage(error),
      "warning",
    );
    return false;
  } finally {
    loading.value = false;
  }
}

async function importWorkbook() {
  const path = await open({
    title: translate("dialog.workbookPicker"),
    multiple: false,
    filters: [{ name: "Excel Macro Workbook", extensions: ["xlsm", "xlsx"] }],
  });
  if (!path || Array.isArray(path)) return false;
  try {
    applySnapshot(await backend.importWorkbook(path));
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

async function addMember(input: AddMemberInput, options: PostingOptions = {}) {
  try {
    applySnapshot(
      await backend.addMember({
        ...input,
        channel: options.channel ?? Channel.Cash,
        ...operationalTimestamp(options.businessDate),
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

async function previewLoan(input: CreateLoanInput) {
  return backend.previewLoan(input);
}

/** Dengan `disburse`, pinjaman langsung dicairkan (pencatatan dari Buku Kas). */
async function createLoan(input: CreateLoanInput, disburse?: PostingOptions) {
  try {
    applySnapshot(
      await backend.createLoan({
        ...input,
        disbursement: disburse
          ? {
              channel: disburse.channel ?? Channel.Cash,
              ...operationalTimestamp(disburse.businessDate),
            }
          : undefined,
      }),
    );
    if (disburse) {
      notify(
        translate("notifications.loanDisbursed"),
        translate("notifications.loanDisbursedMessage"),
      );
      return true;
    }
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

/** Tanggal hari ini (zona waktu lokal) dalam format YYYY-MM-DD. */
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

async function disburseLoan(loanId: string, options: PostingOptions = {}) {
  try {
    applySnapshot(
      await backend.disburseLoan({
        loanId,
        channel: options.channel ?? Channel.Cash,
        ...operationalTimestamp(options.businessDate),
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

async function postSavingsTransaction(
  input: SavingsTransactionInput,
  options: PostingOptions = {},
) {
  try {
    applySnapshot(
      await backend.postSavingsTransaction({
        ...input,
        channel: options.channel ?? Channel.Cash,
        ...operationalTimestamp(options.businessDate),
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

async function postPayment(input: PaymentInput, options: PostingOptions = {}) {
  const backendInput = {
    ...input,
    channel: options.channel ?? Channel.Cash,
    ...operationalTimestamp(options.businessDate),
  };
  try {
    applySnapshot(await backend.postPayment(backendInput));
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
    const state = await backend.getAdminState();
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
    const state = await backend.saveFinancialParameters(input);
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

async function postCashEntry(
  input: CashEntryInput,
  options: PostingOptions = {},
) {
  try {
    applySnapshot(
      await backend.postCashEntry({
        ...input,
        channel: options.channel ?? Channel.Cash,
        ...operationalTimestamp(options.businessDate),
      }),
    );
    notify(
      translate("notifications.paymentPosted"),
      `${input.category} · ${formatCurrency(input.amount)}`,
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
  return backend.getCashBook(year, channel);
}

async function reverseTransaction(id: string) {
  try {
    applySnapshot(
      await backend.reverseTransaction({ id, ...operationalTimestamp() }),
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

let monthlyLedgerRequest = 0;
async function loadMonthlyLedger() {
  const request = ++monthlyLedgerRequest;
  monthlyLedgerLoading.value = true;
  try {
    const ledger = await backend.getMonthlyLedger(selectedYear.value);
    if (request !== monthlyLedgerRequest) return;
    monthlyLedger.periods = ledger.periods;
    monthlyLedger.savings = ledger.savings;
    monthlyLedger.loans = ledger.loans;
  } catch (error) {
    if (request !== monthlyLedgerRequest) return;
    monthlyLedger.periods = [];
    monthlyLedger.savings = [];
    monthlyLedger.loans = [];
    notify(
      translate("notifications.refreshFailed"),
      errorMessage(error),
      "warning",
    );
  } finally {
    if (request === monthlyLedgerRequest) monthlyLedgerLoading.value = false;
  }
}

let monthlyLedgerWatching = false;
/** Memuat buku besar bulanan dan memperbaruinya saat tahun, perusahaan, atau data berubah. */
export function useMonthlyLedger() {
  if (!monthlyLedgerWatching) {
    monthlyLedgerWatching = true;
    // Scope terpisah agar watcher tetap hidup setelah halaman pertama ditutup.
    effectScope(true).run(() =>
      watch(
        [selectedYear, selectedCompanyId, snapshotVersion],
        loadMonthlyLedger,
      ),
    );
  }
  void loadMonthlyLedger();
  return { monthlyLedger, monthlyLedgerLoading };
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

/** Bulan sebelum periode, mis. "2026-01" -> "Desember 2025". */
export function formatPreviousPeriod(period: string) {
  const [year, month] = period.split("-").map(Number);
  return month === 1
    ? `${monthNames[11]} ${year - 1}`
    : `${monthNames[month - 2]} ${year}`;
}

/** "2026-09-02" -> "02-09-2026"; kosong menjadi "". */
export function formatSheetDate(value: string) {
  const match = value.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  return match ? `${match[3]}-${match[2]}-${match[1]}` : value;
}

/** Angka gaya sheet: titik ribuan, kurung untuk negatif, "-" untuk nol. */
export function formatSheetNumber(value: number) {
  if (!value) return "-";
  const formatted = new Intl.NumberFormat(DEFAULT_LOCALE, {
    maximumFractionDigits: 0,
  }).format(Math.abs(value));
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

export function savingsBalance(
  memberId: string,
  accountType: SavingsAccountType,
  balances: SavingsBalance[],
) {
  return (
    balances.find(
      (balance) =>
        balance.memberId === memberId && balance.accountType === accountType,
    )?.balance ?? 0
  );
}

export function memberSavings(member: Member, balances: SavingsBalance[]) {
  return (
    member.principalSavings +
    savingsBalance(member.id, SavingsAccountType.Mandatory, balances) +
    savingsBalance(member.id, SavingsAccountType.Voluntary, balances)
  );
}

export function memberLoanBalance(memberId: string, memberLoans: Loan[]) {
  return memberLoans
    .filter(
      (loan) =>
        loan.memberId === memberId &&
        (loan.status === LoanStatus.Active ||
          loan.status === LoanStatus.NeedsReview),
    )
    .reduce((sum, loan) => sum + loan.balance, 0);
}

export const useKoperasiStore = () => ({
  members,
  savingsBalances,
  loans,
  transactions,
  totals,
  loading,
  refreshing,
  backendError,
  admin,
  toasts,
  companies,
  selectedCompanyId,
  selectedYear,
  yearOptions,
  yearTransactions,
  yearLoans,
  yearMembers,
  yearTotals,
  yearHasData,
  initialize,
  selectCompany,
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
  postCashEntry,
  getCashBook,
  snapshotVersion,
  loadAdminState,
  saveFinancialParameters,
  activeLoans: computed(() =>
    loans.filter((loan) => loan.status === LoanStatus.Active),
  ),
});
