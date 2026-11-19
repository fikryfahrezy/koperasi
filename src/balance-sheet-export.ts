// The Neraca page as NERACA of BUKU KAS HARIAN: AKTIVA in A–C and PASIVA in
// D–F, a column per date (31 Desember of the year before and the month shown).
// The lines are values; each Jumlah is a SUM of its lines and TOTAL AKTIVA /
// TOTAL PASIVA add the Jumlah cells, as in the sheet.
import type { SheetRow } from "./balance-sheet";
import type { BalanceSheetDetail } from "./store/koperasi";
import { SheetBuilder, cellRef, type SheetSpec } from "./xlsx-sheet";

const FIRST_ROW = 7;
const LAST_COL = 6;
const sides = [
  { label: 1, amounts: [2, 3] },
  { label: 4, amounts: [5, 6] },
] as const;

/** `YYYY-MM-DD` of the last day of a `YYYY-MM` period. */
export function lastDate(period: string) {
  const [year, month] = period.split("-").map(Number);
  const day = new Date(year, month, 0).getDate();
  return `${period}-${String(day).padStart(2, "0")}`;
}

export function balanceSheetSheet(input: {
  company: string;
  /** "31 JANUARI 2026". */
  position: string;
  /** `YYYY-MM` of each amount column. */
  periods: [string, string];
  rows: SheetRow[];
  /** Catatan under the Neraca. */
  notes: {
    nonMemberByRecommender: { recommender: string; amount: number }[];
    /** Piutang Lainnya: a bon per entry, in the order typed. */
    otherLoans: BalanceSheetDetail[];
  };
  fileName: string;
}): SheetSpec {
  const sheet = new SheetBuilder();

  [input.company, "NERACA", `POSISI ${input.position}`].forEach((text, i) => {
    sheet.text(i + 1, 1, text, "title");
    sheet.merge(i + 1, 1, i + 1, LAST_COL);
  });

  sheet.header(5, 1, 5, 3, "AKTIVA");
  sheet.header(5, 4, 5, 6, "PASIVA");

  for (const side of sides) {
    sheet.text(6, side.label, "URAIAN", "header");
    side.amounts.forEach((col, index) =>
      sheet.date(6, col, lastDate(input.periods[index])),
    );
  }

  // Per side: the rows of the section's lines, and the rows of its Jumlah.
  const lineRows = sides.map(() => [] as number[]);
  const totalRows = sides.map(() => [] as number[]);

  input.rows.forEach((row, index) => {
    const r = FIRST_ROW + index;
    const grand = index === input.rows.length - 1;

    [row.left, row.right].forEach((cell, sideIndex) => {
      const side = sides[sideIndex];

      if (cell.kind === "heading") {
        lineRows[sideIndex] = [];
        sheet.text(r, side.label, cell.label, "totalText");
        side.amounts.forEach((col) => sheet.blank(r, col, "number"));
      } else if (cell.kind === "line") {
        lineRows[sideIndex].push(r);
        sheet.text(r, side.label, cell.label);
        side.amounts.forEach((col, column) =>
          sheet.number(r, col, cell.amounts[column] ?? 0),
        );
      } else if (cell.kind === "total") {
        sheet.text(r, side.label, cell.label, "totalText");

        // TOTAL AKTIVA / PASIVA adds the Jumlah above it; a Jumlah sums its lines.
        const summed = grand ? totalRows[sideIndex] : lineRows[sideIndex];
        if (!grand) totalRows[sideIndex].push(r);

        side.amounts.forEach((col, column) => {
          const value = cell.amounts[column] ?? 0;

          if (!summed.length) sheet.number(r, col, value, "totalNumber");
          else
            sheet.formula(
              r,
              col,
              grand
                ? summed.map((at) => cellRef(at, col)).join("+")
                : `SUM(${cellRef(summed[0], col)}:${cellRef(summed[summed.length - 1], col)})`,
              value,
              "totalNumber",
            );
        });
      } else {
        sheet.blank(r, side.label, "text");
        side.amounts.forEach((col) => sheet.blank(r, col, "number"));
      }
    });
  });

  catatan(sheet, FIRST_ROW + input.rows.length + 2, input.notes);

  [40, 15, 15, 40, 15, 15].forEach((width, index) =>
    sheet.width(index + 1, width),
  );

  sheet.freeze = [FIRST_ROW, 1];

  return sheet.build(input.fileName, "NERACA");
}

const sumOf = (values: number[]) =>
  values.reduce((total, value) => total + value, 0);

/** Catatan, as under NERACA: Piutang Non Anggota per rekomendasi (A–B) and
 * Piutang Lainnya per person (D–E), then Bukti Tanggal Pinjam (A–C): each bon
 * with its date, a Jumlah per person, and the total. A person's Piutang
 * Lainnya points at their Jumlah in Bukti Tanggal Pinjam. */
function catatan(
  sheet: SheetBuilder,
  start: number,
  notes: Parameters<typeof balanceSheetSheet>[0]["notes"],
) {
  sheet.text(start, 1, "Catatan :", "totalText");

  // Piutang Non Anggota (Rekomendasi).
  const nonMember = notes.nonMemberByRecommender;
  const head = start + 1;

  sheet.text(head, 1, "Piutang Non Anggota (Rekomendasi)", "totalText");
  nonMember.forEach((entry, index) => {
    const label = entry.recommender || "Belum ada rekomendasi";
    sheet.text(head + 1 + index, 1, `${index + 1}. ${label}`);
    sheet.number(head + 1 + index, 2, entry.amount);
  });

  const nonMemberTotal = head + 1 + nonMember.length;
  const nonMemberSum = sumOf(nonMember.map((entry) => entry.amount));

  if (nonMember.length)
    sheet.formula(
      nonMemberTotal,
      2,
      `SUM(${cellRef(head + 1, 2)}:${cellRef(nonMemberTotal - 1, 2)})`,
      nonMemberSum,
      "totalNumber",
    );
  else sheet.number(nonMemberTotal, 2, 0, "totalNumber");

  // The bon grouped by person, in order of first appearance.
  const people = [
    ...notes.otherLoans
      .reduce(
        (groups, detail) =>
          groups.set(detail.name, [...(groups.get(detail.name) ?? []), detail]),
        new Map<string, BalanceSheetDetail[]>(),
      )
      .entries(),
  ];

  // Bukti Tanggal Pinjam, below both lists.
  let r = Math.max(nonMemberTotal, head + 1 + people.length) + 3;
  sheet.text(r, 1, "Bukti Tanggal Pinjam :", "totalText");
  r += 1;

  // The cell holding each person's amount.
  const personCells: { ref: string; amount: number }[] = [];

  people.forEach(([name, details], index) => {
    const first = r;

    details.forEach((detail, line) => {
      const label =
        line === 0
          ? `${index + 1}. ${name}${detail.note ? ` (${detail.note})` : ""}`
          : detail.note;

      sheet.text(r, 1, label);
      sheet.number(r, 2, detail.amount);
      if (detail.loanDate) sheet.date(r, 3, detail.loanDate);
      else sheet.blank(r, 3, "date");
      r += 1;
    });

    const amount = sumOf(details.map((detail) => detail.amount));

    if (details.length === 1) {
      personCells.push({ ref: cellRef(first, 2), amount });
      return;
    }

    sheet.blank(r, 1, "totalText");
    sheet.formula(
      r,
      2,
      `SUM(${cellRef(first, 2)}:${cellRef(r - 1, 2)})`,
      amount,
      "totalNumber",
    );
    personCells.push({ ref: cellRef(r, 2), amount });
    r += 1;
  });

  const total = sumOf(personCells.map((cell) => cell.amount));
  const totalFormula = personCells.map((cell) => cell.ref).join("+");

  r += 1;
  sheet.blank(r, 1, "totalText");
  if (totalFormula) sheet.formula(r, 2, totalFormula, total, "totalNumber");
  else sheet.number(r, 2, 0, "totalNumber");

  // Piutang Lainnya (Bon Sementara) terdiri, beside Piutang Non Anggota.
  sheet.text(head, 4, "Piutang Lainnya (Bon Sementara) terdiri :", "totalText");
  people.forEach(([name], index) => {
    sheet.text(head + 1 + index, 4, `${index + 1}. ${name}`);
    sheet.formula(
      head + 1 + index,
      5,
      personCells[index].ref,
      personCells[index].amount,
    );
  });

  const otherTotal = head + 1 + people.length;

  if (people.length)
    sheet.formula(
      otherTotal,
      5,
      `SUM(${cellRef(head + 1, 5)}:${cellRef(otherTotal - 1, 5)})`,
      total,
      "totalNumber",
    );
  else sheet.number(otherTotal, 5, 0, "totalNumber");
}
