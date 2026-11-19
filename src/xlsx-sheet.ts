// A worksheet a page lays out itself (SHU, Neraca), sent to `export_sheet`.
// Formulas carry the value they give, so the file shows the app's numbers even
// before Excel recalculates.

export type SheetStyle =
  | "header"
  | "number"
  | "date"
  | "text"
  | "totalNumber"
  | "totalText"
  | "title"
  | "percent"
  | "integer"
  | "headerPercent";

export interface SheetCell {
  row: number;
  col: number;
  text?: string;
  number?: number;
  /** `YYYY-MM-DD`. */
  date?: string;
  /** Without `=`; `number` is the value it gives. */
  formula?: string;
  style: SheetStyle;
}

export interface SheetSpec {
  fileName: string;
  sheetName: string;
  cells: SheetCell[];
  /** [first row, first col, last row, last col], 1-based. */
  merges: [number, number, number, number][];
  columns: { col: number; width: number }[];
  freeze?: [number, number];
}

/** "A", "B", …, "Z", "AA", … for a 1-based column. */
export function columnName(col: number) {
  let name = "";

  for (let rest = col; rest > 0; rest = Math.floor((rest - 1) / 26))
    name = String.fromCharCode(65 + ((rest - 1) % 26)) + name;

  return name;
}

export const cellRef = (row: number, col: number) => `${columnName(col)}${row}`;

/** `$C$3`: a rate in the header, kept when the formula is copied down. */
export const fixedRef = (row: number, col: number) =>
  `$${columnName(col)}$${row}`;

export class SheetBuilder {
  readonly cells: SheetCell[] = [];
  readonly merges: SheetSpec["merges"] = [];
  readonly columns: SheetSpec["columns"] = [];
  freeze?: [number, number];

  text(row: number, col: number, text: string, style: SheetStyle = "text") {
    this.cells.push(text ? { row, col, text, style } : { row, col, style });
  }

  number(
    row: number,
    col: number,
    value: number,
    style: SheetStyle = "number",
  ) {
    this.cells.push({ row, col, number: value, style });
  }

  date(row: number, col: number, iso: string, style: SheetStyle = "date") {
    this.cells.push({ row, col, date: iso, style });
  }

  formula(
    row: number,
    col: number,
    formula: string,
    value: number,
    style: SheetStyle = "number",
  ) {
    this.cells.push({ row, col, formula, number: value, style });
  }

  /** An empty cell that keeps the style's borders. */
  blank(row: number, col: number, style: SheetStyle) {
    this.cells.push({ row, col, style });
  }

  merge(firstRow: number, firstCol: number, lastRow: number, lastCol: number) {
    if (firstRow !== lastRow || firstCol !== lastCol)
      this.merges.push([firstRow, firstCol, lastRow, lastCol]);
  }

  /** A header over `first..last`, every covered cell bordered. */
  header(
    firstRow: number,
    firstCol: number,
    lastRow: number,
    lastCol: number,
    label: string,
  ) {
    for (let row = firstRow; row <= lastRow; row++)
      for (let col = firstCol; col <= lastCol; col++)
        if (row !== firstRow || col !== firstCol)
          this.blank(row, col, "header");

    this.text(firstRow, firstCol, label, "header");
    this.merge(firstRow, firstCol, lastRow, lastCol);
  }

  width(col: number, width: number) {
    this.columns.push({ col, width });
  }

  build(fileName: string, sheetName: string): SheetSpec {
    return {
      fileName,
      sheetName,
      cells: this.cells,
      merges: this.merges,
      columns: this.columns,
      freeze: this.freeze,
    };
  }
}
