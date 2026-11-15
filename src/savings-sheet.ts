// SIMPANAN as in the working sheet: one row per member per month, with
// Pokok, Wajib and Manasuka. See notes/rules-and-formulas.md (Simpanan).
import {
  matchesMember,
  type Member,
  type SavingsMonth,
} from "./store/koperasi";

type NumericKey = {
  [K in keyof SavingsMonth]: SavingsMonth[K] extends number ? K : never;
}[keyof SavingsMonth];

const numericKeys: NumericKey[] = [
  "principalOpening",
  "mandatoryOpening",
  "voluntaryOpening",
  "principalIn",
  "principalOut",
  "mandatoryIn",
  "mandatoryOut",
  "voluntaryIn",
  "voluntaryOut",
  "shu",
  "principalClosing",
  "mandatoryClosing",
  "voluntaryClosing",
];

export type SavingsTotals = Record<NumericKey, number>;

export const openingKeys = [
  "principalOpening",
  "mandatoryOpening",
  "voluntaryOpening",
] as const;

export const closingKeys = [
  "principalClosing",
  "mandatoryClosing",
  "voluntaryClosing",
] as const;

// Ratas Simpanan (kolom DA) and Insentif Simpanan (DC, DD).
export const incentiveKeys = ["average", "incentiveRaw", "incentive"] as const;
/** No and Nama. */
export const leadingColumnWidths = [56, 240];

/** Debit = outflow, Credit = inflow, as in the spreadsheet. The
 * profit-sharing column appears only in the January block (profit sharing
 * for the previous fiscal year). */
export const movementKeysForPeriod = (hasShu: boolean) =>
  (
    [
      "principalOut",
      "principalIn",
      "mandatoryOut",
      "mandatoryIn",
      "voluntaryOut",
      "voluntaryIn",
      "shu",
    ] as const
  ).filter((key) => key !== "shu" || hasShu);

/** The month's rows, following member numbers (M001, M002, ...), narrowed
 * by the search. */
export function savingsRowsForPeriod(
  ledgerRows: readonly SavingsMonth[],
  memberById: ReadonlyMap<string, Member>,
  period: string,
  query: string,
) {
  return ledgerRows
    .filter((row) => row.period === period)
    .sort((a, b) => a.memberId.localeCompare(b.memberId))
    .map((row, index) => ({
      ...row,
      no: index + 1,
      name: memberById.get(row.memberId)?.name ?? row.memberId,
    }))
    .filter((row) =>
      matchesMember(
        query,
        row.name,
        memberById.get(row.memberId)?.memberNumber,
      ),
    );
}

export type SavingsSheetRow = ReturnType<typeof savingsRowsForPeriod>[number];

export function sumSavingsRows(rows: readonly SavingsSheetRow[]) {
  const sum = Object.fromEntries(
    numericKeys.map((key) => [key, 0]),
  ) as SavingsTotals;

  for (const row of rows) {
    for (const key of numericKeys) sum[key] += row[key];
  }

  return sum;
}

export const openingTotal = (row: SavingsTotals) =>
  row.principalOpening + row.mandatoryOpening + row.voluntaryOpening;

export const closingTotal = (row: SavingsTotals) =>
  row.principalClosing + row.mandatoryClosing + row.voluntaryClosing;

/** Ratas and Insentif Simpanan per member as of `period`, computed by the
 * backend (rules 2.3a). */
export function incentivesAt(
  ledgerRows: readonly SavingsMonth[],
  period: string,
) {
  return new Map(
    ledgerRows
      .filter((row) => row.period === period)
      .map((row) => [
        row.memberId,
        {
          average: row.savingsAverage,
          incentiveRaw: row.savingsIncentiveRaw,
          incentive: row.savingsIncentive,
        },
      ]),
  );
}
