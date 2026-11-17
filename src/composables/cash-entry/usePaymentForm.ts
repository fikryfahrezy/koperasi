import { computed, watch } from "vue";

import {
  borrowerKey,
  installmentOf,
  savingsAccountsText,
  savingsAmountsOf,
  sumAmounts,
} from "../../cash-entry";

import {
  formatCurrency,
  formatLoanLabel,
  formatPeriod,
  type Loan,
  LoanGroup,
  type PostingOptions,
  useKoperasiStore,
  useMonthlyLedger,
} from "../../store/koperasi";

import type { WorkspacePageState } from "../useWorkspacePageState";

const emptyPayment = () => ({
  loanGroup: LoanGroup.Member as LoanGroup,
  memberId: "",
  // Non Anggota: the borrower, by name (see `borrowerKey`).
  borrower: "",
  // Principal and interest per running loan of the member or borrower.
  lines: {} as Record<string, { principal: number; interest: number }>,
  pokok: 0,
  wajib: 0,
  voluntary: 0,
});

/** Setoran anggota: pokok and bunga for each running loan of a member or Non
 * Anggota borrower, plus savings for a member, as one Buku Kas row ("Setoran
 * <nama>"). A member without a running loan deposits only savings. */
export function usePaymentForm(
  pageState: WorkspacePageState,
  businessDate: () => string,
) {
  const { members, loans, postPayment } = useKoperasiStore();
  const paymentForm = pageState.field("paymentForm", emptyPayment);

  const isNonMemberPayment = computed(
    () => paymentForm.value.loanGroup === LoanGroup.NonMember,
  );

  const runningLoans = computed(() =>
    loans.filter(
      (loan) =>
        loan.loanGroup === paymentForm.value.loanGroup &&
        loan.balance > 0 &&
        (isNonMemberPayment.value || loan.memberId),
    ),
  );

  const paymentBorrowerOptions = computed(() =>
    [
      ...runningLoans.value.reduce((groups, loan) => {
        const key = borrowerKey(loan.memberName);
        groups.set(key, [...(groups.get(key) ?? []), loan]);
        return groups;
      }, new Map<string, Loan[]>()),
    ]
      .map(([key, borrowerLoans]) => {
        const [first] = borrowerLoans;

        const balance = borrowerLoans.reduce(
          (sum, loan) => sum + loan.balance,
          0,
        );

        return {
          value: key,
          label: `${first.memberName.trim()} · ${
            borrowerLoans.length > 1
              ? `${borrowerLoans.length} pinjaman · sisa ${formatCurrency(balance)}`
              : formatLoanLabel(first)
          }`,
        };
      })
      .sort((a, b) => a.label.localeCompare(b.label, "id-ID")),
  );

  // Every member: one without a running loan deposits only savings.
  const paymentMemberOptions = computed(() =>
    members.map((member) => ({
      value: member.id,
      label: member.name,
      code: member.memberNumber,
    })),
  );

  watch(
    () => paymentForm.value.loanGroup,
    () => {
      const form = paymentForm.value;

      if (
        !paymentBorrowerOptions.value.some(
          (option) => option.value === form.borrower,
        )
      )
        form.borrower = "";

      if (
        !paymentMemberOptions.value.some(
          (option) => option.value === form.memberId,
        )
      )
        form.memberId = "";
    },
  );

  // The loans this payment goes to: every running loan of the picked member or
  // Non Anggota borrower.
  const paymentLoans = computed(() =>
    isNonMemberPayment.value
      ? runningLoans.value.filter(
          (loan) =>
            !!paymentForm.value.borrower &&
            borrowerKey(loan.memberName) === paymentForm.value.borrower,
        )
      : runningLoans.value.filter(
          (loan) =>
            paymentForm.value.memberId &&
            loan.memberId === paymentForm.value.memberId,
        ),
  );

  // Interest of the transaction month: this month's installment ("Kewajiban
  // setor tiap bulan") and what is still owed through it ("Kewajiban setor
  // s/d" on the Pinjaman page, less what was already paid that month).
  const { monthlyLedger } = useMonthlyLedger();
  const postingPeriod = computed(() => businessDate().slice(0, 7));

  const interestOfMonth = computed(
    () =>
      new Map(
        monthlyLedger.value.loans
          .filter((row) => row.period === postingPeriod.value)
          .map((row) => [
            row.loanId,
            { scheduled: row.scheduledInterest, due: row.arrearsInterest },
          ]),
      ),
  );

  function interestDueLines(loanIds: string[]) {
    let scheduled = 0;
    let due = 0;

    for (const loanId of loanIds) {
      const interest = interestOfMonth.value.get(loanId);
      if (!interest) return [];
      scheduled += interest.scheduled;
      due += interest.due;
    }

    if (!loanIds.length) return [];

    const dueText =
      due < 0
        ? `Lebih bayar bunga ${formatCurrency(-due)}`
        : `Kewajiban bunga s/d ${formatPeriod(postingPeriod.value)} ${formatCurrency(due)}`;

    return [`Bunga per bulan ${formatCurrency(scheduled)}`, dueText];
  }

  /** A payment that pays off the loan's principal but not exactly its interest
   * due leaves interest arrears on a paid-off loan; warn, but still allow it
   * (the interest may be waived). */
  function payoffInterestWarning(
    loan: Loan,
    principal: number,
    interest: number,
  ) {
    const due = interestOfMonth.value.get(loan.id)?.due;

    if (due === undefined || loan.balance <= 0 || principal !== loan.balance)
      return "";

    const gap = due - (interest || 0);
    if (!gap) return "";

    return gap > 0
      ? `Pinjaman lunas, tapi bunga masih kurang ${formatCurrency(gap)}.`
      : `Pinjaman lunas, tapi bunga lebih ${formatCurrency(-gap)}.`;
  }

  function paymentLine(loanId: string) {
    paymentForm.value.lines ??= {};
    return (paymentForm.value.lines[loanId] ??= { principal: 0, interest: 0 });
  }

  // Picking the member or borrower starts every loan at its installment.
  watch(
    [
      () => pageState.tabId.value,
      () => paymentForm.value.memberId,
      () => paymentForm.value.borrower,
      () => paymentForm.value.loanGroup,
    ],
    ([tabId], [previousTabId]) => {
      if (tabId !== previousTabId) return;

      paymentForm.value.lines = Object.fromEntries(
        paymentLoans.value.map((loan) => [loan.id, installmentOf(loan)]),
      );
    },
  );

  const paymentSavingsAmounts = computed(() =>
    savingsAmountsOf({
      principal: paymentForm.value.pokok,
      mandatory: paymentForm.value.wajib,
      voluntary: paymentForm.value.voluntary,
    }),
  );

  /** An Anggota setoran with no pokok or bunga for any loan: only savings. */
  const isSavingsOnlyPayment = computed(
    () =>
      !isNonMemberPayment.value &&
      paymentLoans.value.every(
        (loan) =>
          !(paymentLine(loan.id).principal || 0) &&
          !(paymentLine(loan.id).interest || 0),
      ),
  );

  const paymentTotal = computed(
    () =>
      paymentLoans.value.reduce(
        (sum, loan) =>
          sum +
          (paymentLine(loan.id).principal || 0) +
          (paymentLine(loan.id).interest || 0),
        0,
      ) +
      (isNonMemberPayment.value ? 0 : sumAmounts(paymentSavingsAmounts.value)),
  );

  const memberName = (id: string) =>
    members.find((member) => member.id === id)?.name ?? "";

  /** The Uraian the backend writes when none is typed; "" until it is known. */
  const generatedDescription = computed(() => {
    if (isSavingsOnlyPayment.value) {
      const name = memberName(paymentForm.value.memberId);
      const accounts = savingsAccountsText(paymentSavingsAmounts.value);
      return name && accounts ? `Setoran ${accounts} ${name}` : "";
    }

    const loan = paymentLoans.value[0];
    if (!loan) return "";
    return `Setoran ${isNonMemberPayment.value ? loan.memberName : memberName(loan.memberId)}`;
  });

  function reset() {
    const { lines: _lines, ...fields } = emptyPayment();
    Object.assign(paymentForm.value, fields);
  }

  /** Savings only (no pokok or bunga) is posted by the same command. */
  async function submit(options: PostingOptions, description: string) {
    const form = paymentForm.value;
    const nonMember = isNonMemberPayment.value;
    if (nonMember ? !paymentLoans.value[0] : !form.memberId) return false;
    if (paymentTotal.value <= 0) return false;

    return postPayment(
      {
        memberId: nonMember ? "" : form.memberId,
        principal: 0,
        interest: 0,
        loans: paymentLoans.value.map((item) => ({
          loanId: item.id,
          principal: paymentLine(item.id).principal || 0,
          interest: paymentLine(item.id).interest || 0,
        })),
        pokok: nonMember ? 0 : form.pokok || 0,
        wajib: nonMember ? 0 : form.wajib || 0,
        voluntary: nonMember ? 0 : form.voluntary || 0,
        description,
      },
      options,
    );
  }

  return {
    paymentForm,
    isNonMemberPayment,
    paymentBorrowerOptions,
    paymentMemberOptions,
    paymentLoans,
    paymentLine,
    paymentTotal,
    interestDueLines,
    payoffInterestWarning,
    generatedDescription,
    reset,
    submit,
  };
}
