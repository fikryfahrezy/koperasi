// Spreadsheet column letters (A, B, ..., AA) of the 2026 workbooks, shown above
// the Buku Kas, Simpanan and Pinjaman tables so values can be matched with Excel.
// The app repeats some columns every month (e.g. last month's balance); those
// carry the letter of the column the value comes from. "" = not in the sheet.

/** 1 -> "A", 27 -> "AA". */
export function columnLetter(column: number) {
  let letters = "";
  for (let n = column; n > 0; n = Math.floor((n - 1) / 26))
    letters = String.fromCharCode(65 + ((n - 1) % 26)) + letters;
  return letters;
}

const letters = (columns: number[]) =>
  columns.map((column) => (column > 0 ? columnLetter(column) : ""));
const range = (first: number, count: number) =>
  Array.from({ length: count }, (_, index) => first + index);
const monthOf = (period: string) => Number(period.slice(5, 7));

/** TAHUN 2026: Tanggal, Uraian, Debet, Kredit, Saldo, Keluar, Masuk, Kas + Tabungan. */
export const cashBookLetters = letters([1, 2, 5, 6, 7, 9, 10, 11]);

/** PINJAMAN BULANAN: No, Nama, Plafond, Bunga (2), PG/TN, Jenis, Realisasi, Jatuh Tempo, Jangka Waktu. */
export const loanLeadingLetters = letters(range(1, 10));

// Saldo Posisi Desember (K), Kewajiban setor tiap bulan (L–N) and Tunggakan
// s/d Desember (O–Q) come once, before January. Each month is then Kewajiban
// s/d (3), Mutasi (6), Saldo (1), Tunggakan (3), starting at R for January;
// the last column is ET (Kewajiban s/d November), after which the income
// columns begin.
const LOAN_LAST_COLUMN = 150;

/** Saldo Posisi, Kewajiban setor tiap bulan and Tunggakan before the first month. */
export const loanOpeningLetters = letters(range(11, 7));

/** The 13 columns of one month on the Pinjaman page. */
export function loanMonthLetters(period: string) {
  const start = 18 + 13 * (monthOf(period) - 1);
  return letters(
    range(start, 13).map((column) => (column > LOAN_LAST_COLUMN ? 0 : column)),
  );
}

/** After the last month: Kewajiban s/d bulan depan, Rekomendasi (EV),
 * Jumlah pemasukan per triwulan (Okt s/d Des is not in the sheet yet) and
 * Insentif Pinjaman. */
export function loanTailLetters(nextPeriod: string) {
  return [
    ...loanMonthLetters(nextPeriod).slice(0, 3), // Kewajiban s/d bulan depan
    ...letters([
      152, // EV Rekomendasi
      ...range(154, 3), // EX–EZ
      ...range(158, 3), // FB–FD
      ...range(162, 3), // FF–FH
      0,
      0,
      0,
      166, // FJ
      167, // FK
    ]),
  ];
}

/** DATA SIMPANAN: No, Nama Pensiunan. */
export const savingsLeadingLetters = letters([1, 2]);

// Month blocks of DATA SIMPANAN TAHUN 2026 are not uniform: January and March
// have one column each for Pokok and Wajib, and January has the SHU column.
const savingsMonths: Record<
  number,
  { date: number; pokok: number[]; wajib: number[]; next: number }
> = {
  1: { date: 7, pokok: [0, 8], wajib: [0, 9], next: 10 },
  2: { date: 17, pokok: [18, 19], wajib: [20, 21], next: 22 },
  3: { date: 28, pokok: [0, 29], wajib: [0, 30], next: 31 },
  4: { date: 37, pokok: [38, 39], wajib: [40, 41], next: 42 },
  5: { date: 48, pokok: [49, 50], wajib: [51, 52], next: 53 },
  6: { date: 59, pokok: [60, 61], wajib: [62, 63], next: 64 },
  7: { date: 70, pokok: [71, 72], wajib: [73, 74], next: 75 },
  8: { date: 81, pokok: [82, 83], wajib: [84, 85], next: 86 },
  9: { date: 92, pokok: [93, 94], wajib: [95, 96], next: 97 },
  10: { date: 103, pokok: [104, 105], wajib: [106, 107], next: 108 },
};

/** Saldo Pokok column of a month; Wajib, Manasuka and the total follow it. */
function savingsBalanceColumn(month: number) {
  if (month === 0) return 3; // SALDO BULAN DESEMBER 2025: C, D, E, total F
  const block = savingsMonths[month];
  if (!block) return 0;
  return block.next + 2 + (month === 1 ? 1 : 0);
}

/** Saldo (3) + total of the month before the first month shown, once before it. */
export function savingsOpeningLetters(firstPeriod: string, year: number) {
  const month = monthOf(firstPeriod);
  const previous =
    year === 2026 && savingsMonths[month] ? savingsBalanceColumn(month - 1) : 0;
  return previous ? letters(range(previous, 4)) : Array<string>(4).fill("");
}

/** The 11 (12 with SHU) columns of one month on the Simpanan page. */
export function savingsMonthLetters(
  period: string,
  hasShu: boolean,
  year: number,
) {
  const month = monthOf(period);
  const block = year === 2026 ? savingsMonths[month] : undefined;
  if (!block) return Array<string>(hasShu ? 12 : 11).fill("");
  const balance = savingsBalanceColumn(month);
  return letters([
    block.date,
    ...block.pokok,
    ...block.wajib,
    block.next,
    block.next + 1, // Manasuka debet, kredit
    ...(hasShu ? [month === 1 ? block.next + 2 : 0] : []),
    ...range(balance, 4), // Saldo bulan ini (3) + total
  ]);
}

/** After the last month: Ratas Simpanan (DL), Insentif Simpanan 3,65% (DN) and Pembulatan (DO). */
export function savingsTailLetters(year: number) {
  return year === 2026 ? letters([116, 118, 119]) : ["", "", ""];
}
