// The SHU page as DAFTAR SHU TAHUN BUKU (sheet Sheet1 of the RAT workbook),
// with its formulas: SHU = Ratas × Insentif Simpanan, Jasa Pinjaman × Insentif
// Pinjaman, Total = Jumlah SHU + uang duduk, Pembulatan = ROUNDUP(Total; -3).
// Ratas and Jasa Pinjaman are values (the sheet linked them from DATA SIMPANAN
// and DATA PINJAMAN, which Google Sheets cannot open).
import { roundUpThousand, type ShuRow } from "./shu";
import { SheetBuilder, cellRef, fixedRef, type SheetSpec } from "./xlsx-sheet";

const FIRST_ROW = 7;
const LAST_COL = 10;

// Columns A–J of the sheet.
const Col = {
  No: 1,
  Name: 2,
  Average: 3,
  SavingsShu: 4,
  LoanIncome: 5,
  LoanShu: 6,
  Shu: 7,
  Allowance: 8,
  Total: 9,
  Rounded: 10,
} as const;

export function shuSheet(input: {
  year: number;
  rows: (ShuRow & { no: number; name: string })[];
  savingsRate: number;
  loanRate: number;
}): SheetSpec {
  const sheet = new SheetBuilder();

  sheet.text(1, 1, "DAFTAR PENERIMAAN SHU", "title");
  sheet.merge(1, 1, 1, LAST_COL);
  sheet.text(2, 1, `TAHUN BUKU ${input.year}`, "title");
  sheet.merge(2, 1, 2, LAST_COL);

  // The rates the SHU columns multiply by ($C$3 and $F$3).
  sheet.text(3, Col.Name, "Insentif Simpanan", "totalText");
  sheet.number(3, Col.Average, input.savingsRate, "percent");
  sheet.text(3, Col.SavingsShu, "Insentif Pinjaman", "totalText");
  sheet.blank(3, Col.LoanIncome, "totalText");
  sheet.merge(3, Col.SavingsShu, 3, Col.LoanIncome);
  sheet.number(3, Col.LoanShu, input.loanRate, "percent");

  sheet.header(4, Col.No, 6, Col.No, "No.");
  sheet.header(4, Col.Name, 6, Col.Name, "Nama Anggota");
  sheet.header(4, Col.Average, 4, Col.SavingsShu, "Simpanan");
  sheet.header(5, Col.Average, 6, Col.Average, "Ratas");
  sheet.header(5, Col.SavingsShu, 6, Col.SavingsShu, "SHU");
  sheet.header(4, Col.LoanIncome, 4, Col.LoanShu, "Pinjaman");
  sheet.header(5, Col.LoanIncome, 6, Col.LoanIncome, "Jasa Pinjaman");
  sheet.header(5, Col.LoanShu, 6, Col.LoanShu, "SHU");
  sheet.header(4, Col.Shu, 6, Col.Shu, "Jumlah SHU");
  sheet.header(4, Col.Allowance, 6, Col.Allowance, "Uang Duduk + Uang Makan");
  sheet.header(4, Col.Total, 6, Col.Total, "Total");
  sheet.header(4, Col.Rounded, 6, Col.Rounded, "Pembulatan");

  const savingsRate = fixedRef(3, Col.Average);
  const loanRate = fixedRef(3, Col.LoanShu);

  input.rows.forEach((row, index) => {
    const r = FIRST_ROW + index;
    const at = (col: number) => cellRef(r, col);

    sheet.number(r, Col.No, row.no, "integer");
    sheet.text(r, Col.Name, row.name);
    sheet.number(r, Col.Average, row.savingsAverage);
    sheet.formula(
      r,
      Col.SavingsShu,
      `${at(Col.Average)}*${savingsRate}`,
      row.savingsShu,
    );
    sheet.number(r, Col.LoanIncome, row.loanIncome);
    sheet.formula(
      r,
      Col.LoanShu,
      `${at(Col.LoanIncome)}*${loanRate}`,
      row.loanShu,
    );
    sheet.formula(
      r,
      Col.Shu,
      `${at(Col.SavingsShu)}+${at(Col.LoanShu)}`,
      row.shu,
    );
    sheet.number(r, Col.Allowance, row.sittingAllowance);
    sheet.formula(
      r,
      Col.Total,
      `${at(Col.Shu)}+${at(Col.Allowance)}`,
      row.total,
    );
    sheet.formula(
      r,
      Col.Rounded,
      `ROUNDUP(${at(Col.Total)},-3)`,
      roundUpThousand(row.total),
    );
  });

  // JUMLAH: SUM of every column, as in the sheet.
  const totalRow = FIRST_ROW + input.rows.length;
  const lastRow = totalRow - 1;

  sheet.blank(totalRow, Col.No, "totalText");
  sheet.text(totalRow, Col.Name, "JUMLAH", "totalText");

  for (let col: number = Col.Average; col <= LAST_COL; col++) {
    const keys: Record<number, keyof ShuRow> = {
      [Col.Average]: "savingsAverage",
      [Col.SavingsShu]: "savingsShu",
      [Col.LoanIncome]: "loanIncome",
      [Col.LoanShu]: "loanShu",
      [Col.Shu]: "shu",
      [Col.Allowance]: "sittingAllowance",
      [Col.Total]: "total",
      [Col.Rounded]: "rounded",
    };

    const key = keys[col];
    const value = input.rows.reduce(
      (total, row) => total + Number(row[key]),
      0,
    );

    // Without rows a SUM would point at the JUMLAH row itself.
    if (!input.rows.length) sheet.number(totalRow, col, value, "totalNumber");
    else
      sheet.formula(
        totalRow,
        col,
        `SUM(${cellRef(FIRST_ROW, col)}:${cellRef(lastRow, col)})`,
        value,
        "totalNumber",
      );
  }

  [5.86, 36.29, 14.29, 12.43, 14.29, 12.43, 12.29, 14, 12, 13].forEach(
    (width, index) => sheet.width(index + 1, width),
  );

  sheet.freeze = [FIRST_ROW, Col.Average];

  return sheet.build(`DAFTAR SHU TAHUN BUKU ${input.year}.xlsx`, "Sheet1");
}
