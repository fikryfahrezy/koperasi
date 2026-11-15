// Shared rules of Catat transaksi (CashEntryDialog): how loans are grouped
// by borrower, the installment to start from, and the savings wording.
import {
  InterestType,
  type Loan,
  roundUpInterest,
  SavingsAccountType,
} from "./store/koperasi";

/** Non Anggota borrowers have no member record, so their loans are grouped by
 * name, ignoring case and outer spaces (the backend matches the same way). */
export const borrowerKey = (name: string) =>
  name.trim().toLocaleLowerCase("id-ID");

/** One month's installment, to start from: the outstanding balance helps
 * fill in the principal. */
export function installmentOf(loan: Loan) {
  return {
    principal: Math.min(
      Math.ceil(loan.plafond / loan.tenor / 1000) * 1000,
      loan.balance,
    ),
    interest: roundUpInterest(
      ((loan.interestType === InterestType.Flat ? loan.plafond : loan.balance) *
        loan.rate) /
        100 /
        12,
    ),
  };
}

export interface SavingsAmount {
  accountType: SavingsAccountType;
  amount: number;
}

/** Pokok, Wajib and Manasuka as typed; only those above zero are posted. */
export function savingsAmountsOf(amounts: {
  principal: number;
  mandatory: number;
  voluntary: number;
}): SavingsAmount[] {
  return (
    [
      [SavingsAccountType.Principal, amounts.principal],
      [SavingsAccountType.Mandatory, amounts.mandatory],
      [SavingsAccountType.Voluntary, amounts.voluntary],
    ] as const
  )
    .map(([accountType, amount]) => ({ accountType, amount: amount || 0 }))
    .filter((item) => item.amount > 0);
}

export const sumAmounts = (items: readonly { amount: number }[]) =>
  items.reduce((sum, item) => sum + item.amount, 0);

const savingsAccountWords = {
  [SavingsAccountType.Principal]: "Pokok",
  [SavingsAccountType.Mandatory]: "Wajib",
  [SavingsAccountType.Voluntary]: "Manasuka",
} satisfies Record<SavingsAccountType, string>;

/** "Simpanan Wajib" or "Simpanan Pokok, Wajib & Manasuka", as the backend writes it. */
export function savingsAccountsText(
  amounts: readonly { accountType: SavingsAccountType }[],
) {
  const words = amounts.map((item) => savingsAccountWords[item.accountType]);
  if (!words.length) return "";

  return words.length === 1
    ? `Simpanan ${words[0]}`
    : `Simpanan ${words.slice(0, -1).join(", ")} & ${words[words.length - 1]}`;
}
