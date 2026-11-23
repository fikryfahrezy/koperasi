// Rows of Buku Kas as the Kas page shows them: plain words for a movement,
// the search, and the Riwayat hapus list of deleted rows.
import {
  type CashBookRow,
  formatCurrency,
  formatSavedAt,
  formatSheetNumber,
  type Loan,
  type Transaction,
  TransactionDirection,
  TransactionStatus,
} from "./store/koperasi";

const itemWords: Record<string, string> = {
  LOAN_PRINCIPAL: "pokok",
  LOAN_INTEREST: "bunga",
  LOAN_PROVISION: "provisi",
  LOAN_DISBURSEMENT: "pencairan",
  SAVINGS_DEPOSIT: "setor",
  SAVINGS_WITHDRAWAL: "ambil",
  SAVINGS_SHU: "SHU",
};

const accountWords: Record<string, string> = {
  POKOK: "pokok",
  WAJIB: "wajib",
  MANASUKA: "manasuka",
};

/** "Pembayaran pinjaman Rp 13.900.000" with "pokok 10.007.331 · bunga 200.147"
 * for a loan or savings movement; other rows keep their description. */
export function plainWords(transaction: Transaction, loans: readonly Loan[]) {
  const items = transaction.components;

  const loan = loans.find((candidate) =>
    items.some((item) => item.loanId === candidate.id),
  );

  const parts = items.map((item) =>
    item.accountType
      ? `${accountWords[item.accountType] ?? item.accountType} ${formatSheetNumber(item.amount)}`
      : `${itemWords[item.componentType] ?? item.label} ${formatSheetNumber(item.amount)}`,
  );

  const imported = transaction.id.includes("-MIG");

  if (items.some((item) => item.loanId)) {
    const disbursed = items.some(
      (item) => item.componentType === "LOAN_DISBURSEMENT",
    );

    return {
      title: `${disbursed ? "Pencairan" : "Pembayaran"} pinjaman${loan ? ` ${formatCurrency(loan.plafond)}` : ""}`,
      detail: [...parts, imported ? "dari sheet PINJAMAN" : ""]
        .filter(Boolean)
        .join(" · "),
    };
  }

  if (items.some((item) => item.accountType)) {
    const withdrawn = items.some(
      (item) => item.componentType === "SAVINGS_WITHDRAWAL",
    );

    return {
      title: withdrawn ? "Pengambilan simpanan" : "Setoran simpanan",
      detail: [...parts, imported ? "dari sheet SIMPANAN" : ""]
        .filter(Boolean)
        .join(" · "),
    };
  }

  return { title: transaction.description, detail: "" };
}

/** Users see a reversal as a deletion: the reversed row and its reversal
 * are never shown in the sheet. */
export const isExcluded = (row: CashBookRow) =>
  row.status === TransactionStatus.Reversed || row.isReversal;

/** The search matches the Uraian, or the ID anggota once something is typed. */
export function matchesSearch(row: CashBookRow, query: string) {
  const search = query.trim().toLowerCase();

  return (
    row.description.toLowerCase().includes(search) ||
    (search !== "" && row.memberNumber.toLowerCase().includes(search))
  );
}

/** Riwayat hapus: every deleted row of the year, newest deletion first. The
 * reversal row carries when it was saved (the deletion) and the id of the row it cancels. */
export function deletedRowsOf(rows: readonly CashBookRow[]) {
  const reversals = rows.filter((row) => row.isReversal);

  return rows
    .filter((row) => row.status === TransactionStatus.Reversed)
    .map((row) => {
      const reversal = reversals.find(
        (candidate) => candidate.reversedTransactionId === row.id,
      );

      return {
        ...row,
        // General cash rows are booked to "Koperasi": no one to show. A name
        // without an ID Anggota is a Non Anggota borrower.
        // As on the sheet: Debet is cash out, Kredit is cash in.
        summary: [
          `${row.direction === TransactionDirection.Out ? "Debet" : "Kredit"} ${formatCurrency(row.amount)}`,
          row.memberNumber
            ? `${row.memberNumber} ${row.memberName}`
            : row.memberName && row.memberName !== "Koperasi"
              ? `${row.memberName} (Non Anggota)`
              : "",
        ]
          .filter(Boolean)
          .join(" · "),
        deletedAt: reversal ? formatSavedAt(reversal.createdAt) : "",
        deletedOrder: reversal ? `${reversal.createdAt} ${reversal.id}` : "",
      };
    })
    .sort((a, b) => b.deletedOrder.localeCompare(a.deletedOrder));
}

/** KAS BUKU TABUNGAN totals: Keluar and Masuk of the rows shown. */
export function savingsBookTotalsOf(rows: readonly CashBookRow[]) {
  return rows
    .filter((row) => !isExcluded(row))
    .reduce(
      (sum, row) => ({
        out: sum.out + row.savingsBookOut,
        in: sum.in + row.savingsBookIn,
      }),
      { out: 0, in: 0 },
    );
}

/**
 * KAS + TABUNGAN per row: Saldo Kas after the row plus the Kas Buku Tabungan
 * balance so far (opening + Masuk − Keluar). The bank part is an estimate, as
 * on the Neraca: money that reaches the bank outside Buku Kas is not in it.
 */
export function cashPlusSavingsOf(
  rows: readonly CashBookRow[],
  savingsBookOpening: number,
) {
  const totals = new Map<string, number>();
  let bank = savingsBookOpening;

  for (const row of rows) {
    if (isExcluded(row)) continue;
    bank += row.savingsBookIn - row.savingsBookOut;
    totals.set(row.id, row.balance + bank);
  }

  return totals;
}
