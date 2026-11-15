import { computed, watch } from "vue";
import { borrowerKey, installmentOf } from "../../cash-entry";

import {
  Guarantee,
  InterestType,
  LoanGroup,
  type LoanPreview,
  LoanType,
  type PostingOptions,
  useKoperasiStore,
} from "../../store/koperasi";

import type { WorkspacePageState } from "../useWorkspacePageState";

const emptyLoan = () => ({
  loanGroup: LoanGroup.Member as LoanGroup,
  memberId: "",
  borrowerName: "",
  plafond: 10_000_000,
  tenor: null as number | null,
  interestType: InterestType.Declining as InterestType,
  loanType: LoanType.Monthly as LoanType,
  guarantee: Guarantee.Pg as Guarantee,
  recommender: "",
  // Bunga per tahun (%); null follows the Bunga parameter until typed.
  annualRate: null as number | null,
});

export const interestTypeOptions = [
  { value: InterestType.Declining, label: "Menurun" },
  { value: InterestType.Flat, label: "Plat" },
];

export const loanGroupOptions = [
  { value: LoanGroup.Member, label: "Anggota PP BRI" },
  { value: LoanGroup.NonMember, label: "Non Anggota PP BRI" },
];

export const guaranteeOptions = [
  { value: Guarantee.Pg, label: "PG" },
  { value: Guarantee.Tn, label: "TN" },
  { value: Guarantee.None, label: "Kosong" },
];

export const loanTypeOptions = [
  { value: LoanType.Monthly, label: "Bulanan" },
  { value: LoanType.Temporary, label: "Sementara" },
];

/** Realisasi pinjaman: a new loan, optionally a suplesi that pays off the
 * member's running loans, with its Provisi and the backend's preview. */
export function useLoanForm(pageState: WorkspacePageState) {
  const { members, loans, financialParameters, createLoan, previewLoan } =
    useKoperasiStore();

  const loanForm = pageState.field("loanForm", emptyLoan);

  // Provisi follows the parameter rate until the user types their own amount.
  const loanProvision = pageState.field("loanProvision", () => ({
    amount: 0,
    edited: false,
  }));

  // Suplesi: the member's running loans this new loan pays off, with the
  // interest still due on each.
  const loanPayoffs = pageState.field(
    "loanPayoffs",
    () => ({}) as Record<string, { selected: boolean; interest: number }>,
  );

  const loanPreview = pageState.field<LoanPreview | null>(
    "loanPreview",
    () => null,
  );

  const topUpLoans = computed(() =>
    loanForm.value.loanGroup === LoanGroup.Member && loanForm.value.memberId
      ? loans.filter(
          (loan) =>
            loan.memberId === loanForm.value.memberId &&
            loan.loanGroup === LoanGroup.Member &&
            loan.balance > 0,
        )
      : [],
  );

  function loanPayoff(loanId: string) {
    return (loanPayoffs.value[loanId] ??= { selected: false, interest: 0 });
  }

  watch(
    [() => pageState.tabId.value, () => loanForm.value.memberId],
    ([tabId], [previousTabId]) => {
      if (tabId !== previousTabId) return;

      loanPayoffs.value = Object.fromEntries(
        topUpLoans.value.map((loan) => [
          loan.id,
          { selected: false, interest: installmentOf(loan).interest },
        ]),
      );
    },
  );

  const selectedPayoffs = computed(() =>
    topUpLoans.value
      .filter((loan) => loanPayoff(loan.id).selected)
      .map((loan) => ({
        loanId: loan.id,
        principal: loan.balance,
        interest: loanPayoff(loan.id).interest || 0,
      })),
  );

  const payoffTotal = computed(() =>
    selectedPayoffs.value.reduce(
      (sum, payoff) => sum + payoff.principal + payoff.interest,
      0,
    ),
  );

  const exceedsTopUpPlafond = computed(
    () =>
      selectedPayoffs.value.length > 0 &&
      payoffTotal.value + loanProvision.value.amount > loanForm.value.plafond,
  );

  /** Batas sisa pinjaman: the borrower's remaining principal on running loans,
   * less what this suplesi pays off, plus the new plafond. Only a warning, since
   * the board can allow more (notes/rules-and-formulas.md 1.12). Non Anggota
   * loans are matched by borrower name. */
  const borrowerOutstanding = computed(() => {
    const form = loanForm.value;
    const name = borrowerKey(form.borrowerName);

    return loans
      .filter(
        (loan) =>
          loan.balance > 0 &&
          loan.loanGroup === form.loanGroup &&
          (form.loanGroup === LoanGroup.Member
            ? !!form.memberId && loan.memberId === form.memberId
            : !!name && borrowerKey(loan.memberName) === name),
      )
      .reduce((sum, loan) => sum + loan.balance, 0);
  });

  const outstandingAfterLoan = computed(
    () =>
      borrowerOutstanding.value -
      selectedPayoffs.value.reduce((sum, payoff) => sum + payoff.principal, 0) +
      (loanForm.value.plafond || 0),
  );

  const exceedsOutstandingLimit = computed(
    () =>
      financialParameters.maxLoanOutstanding > 0 &&
      outstandingAfterLoan.value > financialParameters.maxLoanOutstanding,
  );

  const loanAnnualRate = computed({
    get: () => loanForm.value.annualRate ?? financialParameters.annualRate,
    set: (value: number) => {
      loanForm.value.annualRate = Number.isFinite(value) ? value : null;
    },
  });

  const annualRateValid = computed(
    () => loanAnnualRate.value >= 0 && loanAnnualRate.value <= 100,
  );

  /** Provisi at the parameter rate. A suplesi charges it on the new money
   * only, as in the sheet: plafond minus what pays off the old loans. */
  function rateProvision(preview: LoanPreview | null) {
    if (!payoffTotal.value) return preview?.provision ?? 0;
    const base = Math.max(loanForm.value.plafond - payoffTotal.value, 0);
    return Math.round((base * financialParameters.provisionRate) / 100);
  }

  watch(
    [() => pageState.tabId.value, payoffTotal],
    ([tabId], [previousTabId]) => {
      if (tabId !== previousTabId) return;
      loanProvision.value.edited = false;
      loanProvision.value.amount = rateProvision(loanPreview.value);
    },
  );

  watch(
    loanForm,
    async (_, __, onCleanup) => {
      const target = loanPreview.forTab(pageState.tabId.value);
      const provision = loanProvision.forTab(pageState.tabId.value);
      let cancelled = false;

      onCleanup(() => {
        cancelled = true;
      });

      const { tenor } = loanForm.value;

      if (!tenor || tenor <= 0) {
        target.value = null;
        return;
      }

      try {
        const result = await previewLoan({ ...loanForm.value, tenor });
        if (cancelled) return;
        target.value = result;

        if (!provision.value.edited)
          provision.value.amount = rateProvision(result);
      } catch {
        if (!cancelled) target.value = null;
      }
    },
    { deep: true, immediate: true },
  );

  watch(
    () => loanForm.value.plafond,
    () => {
      loanProvision.value.edited = false;
    },
  );

  const memberName = (id: string) =>
    members.find((member) => member.id === id)?.name ?? "";

  /** The Uraian the backend writes when none is typed; "" until it is known. */
  const generatedDescription = computed(() => {
    const name =
      loanForm.value.loanGroup === LoanGroup.Member
        ? memberName(loanForm.value.memberId)
        : loanForm.value.borrowerName.trim();

    if (!name) return "";

    return selectedPayoffs.value.length
      ? `Suplesi ${name} Jk ${loanForm.value.tenor ?? ""} bulan`
      : `Realisasi Pinjaman ${name}`;
  });

  function reset() {
    Object.assign(loanForm.value, emptyLoan());
    Object.assign(loanProvision.value, { amount: 0, edited: false });
    loanPayoffs.value = {};
  }

  async function submit(options: PostingOptions, description: string) {
    const form = loanForm.value;
    const isMember = form.loanGroup === LoanGroup.Member;

    const borrowerMissing = isMember
      ? !form.memberId
      : !form.borrowerName.trim();

    const { tenor } = form;

    if (
      borrowerMissing ||
      form.plafond <= 0 ||
      !tenor ||
      tenor <= 0 ||
      !Number.isFinite(loanProvision.value.amount) ||
      !annualRateValid.value ||
      exceedsTopUpPlafond.value
    )
      return false;

    return createLoan(
      {
        ...form,
        tenor,
        provision: loanProvision.value.amount,
        memberId: isMember ? form.memberId : "",
        borrowerName: form.borrowerName.trim(),
        payoffs: selectedPayoffs.value,
        description,
      },
      options,
    );
  }

  return {
    loanForm,
    loanProvision,
    loanPreview,
    topUpLoans,
    loanPayoff,
    payoffTotal,
    exceedsTopUpPlafond,
    borrowerOutstanding,
    outstandingAfterLoan,
    exceedsOutstandingLimit,
    loanAnnualRate,
    generatedDescription,
    reset,
    submit,
  };
}
