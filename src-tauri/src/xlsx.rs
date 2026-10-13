//! A minimal .xlsx (SpreadsheetML) writer: one worksheet with numbers, text,
//! dates and formulas, a fixed set of styles, merged cells, column widths and
//! frozen panes. Enough to export the Buku Kas, Simpanan and Pinjaman pages in
//! the layout of the cooperative's workbooks without another dependency.
//!
//! Formulas are built as [`Expr`] so the sheet can evaluate them itself: a
//! formula is only written when it gives the value the app shows (see
//! [`Sheet::checked`]); otherwise the cell keeps the app's value.

use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

/// Cell styles, in the order of `cellXfs` in [`STYLES_XML`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Style {
    // cellXfs 0, the workbook's default style; unused cells get it implicitly.
    #[allow(dead_code)]
    Default = 0,
    Header = 1,
    Number = 2,
    Date = 3,
    Text = 4,
    TotalNumber = 5,
    TotalText = 6,
    Title = 7,
    Percent = 8,
    Integer = 9,
    HeaderPercent = 10,
}

#[derive(Clone, Debug)]
enum Value {
    Number(f64),
    Text(String),
    /// Formula text (without `=`) and the value it evaluates to.
    Formula(String, f64),
}

#[derive(Clone, Debug)]
struct Cell {
    value: Value,
    style: Style,
}

/// A formula over the sheet's cells. Rows and columns are 1-based.
#[derive(Clone, Debug)]
pub(crate) enum Expr {
    Cell(u32, u32),
    /// A cell with an absolute column and row (`$D$8`), e.g. a rate in the header.
    Fixed(u32, u32),
    Num(f64),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    /// `SUM(first:last)` over a rectangle.
    Sum((u32, u32), (u32, u32)),
    Round(Box<Expr>, i32),
    RoundUp(Box<Expr>, i32),
}

impl Expr {
    pub(crate) fn cell(row: u32, col: u32) -> Self {
        Self::Cell(row, col)
    }
    pub(crate) fn num(value: f64) -> Self {
        Self::Num(value)
    }
    pub(crate) fn add(self, other: Expr) -> Self {
        Self::Add(Box::new(self), Box::new(other))
    }
    pub(crate) fn sub(self, other: Expr) -> Self {
        Self::Sub(Box::new(self), Box::new(other))
    }
    pub(crate) fn mul(self, other: Expr) -> Self {
        Self::Mul(Box::new(self), Box::new(other))
    }
    pub(crate) fn div(self, other: Expr) -> Self {
        Self::Div(Box::new(self), Box::new(other))
    }
    pub(crate) fn round(self, digits: i32) -> Self {
        Self::Round(Box::new(self), digits)
    }
    pub(crate) fn round_up(self, digits: i32) -> Self {
        Self::RoundUp(Box::new(self), digits)
    }
    /// `a+b+c...` of the given terms; 0 for none.
    pub(crate) fn sum_of(terms: impl IntoIterator<Item = Expr>) -> Self {
        terms
            .into_iter()
            .reduce(Expr::add)
            .unwrap_or(Expr::Num(0.0))
    }

    fn precedence(&self) -> u8 {
        match self {
            Self::Add(..) | Self::Sub(..) => 1,
            Self::Mul(..) | Self::Div(..) => 2,
            _ => 3,
        }
    }

    fn operand(&self, min: u8) -> String {
        if self.precedence() < min {
            format!("({})", self.formula())
        } else {
            self.formula()
        }
    }

    /// Excel formula text, without the leading `=`.
    pub(crate) fn formula(&self) -> String {
        match self {
            Self::Cell(row, col) => cell_ref(*row, *col),
            Self::Fixed(row, col) => format!("${}${row}", column_name(*col)),
            Self::Num(value) => format_number(*value),
            Self::Add(a, b) => format!("{}+{}", a.formula(), b.operand(1)),
            Self::Sub(a, b) => format!("{}-{}", a.formula(), b.operand(2)),
            Self::Mul(a, b) => format!("{}*{}", a.operand(2), b.operand(2)),
            Self::Div(a, b) => format!("{}/{}", a.operand(2), b.operand(3)),
            Self::Sum(first, last) => format!(
                "SUM({}:{})",
                cell_ref(first.0, first.1),
                cell_ref(last.0, last.1)
            ),
            Self::Round(a, digits) => format!("ROUND({},{digits})", a.formula()),
            Self::RoundUp(a, digits) => format!("ROUNDUP({},{digits})", a.formula()),
        }
    }
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// 1 -> "A", 27 -> "AA".
pub(crate) fn column_name(col: u32) -> String {
    let mut name = Vec::new();
    let mut n = col;
    while n > 0 {
        name.push(b'A' + ((n - 1) % 26) as u8);
        n = (n - 1) / 26;
    }
    name.reverse();
    String::from_utf8(name).unwrap_or_default()
}

pub(crate) fn cell_ref(row: u32, col: u32) -> String {
    format!("{}{row}", column_name(col))
}

/// Excel's ROUND/ROUNDUP: half away from zero, and away from zero.
fn round_digits(value: f64, digits: i32, up: bool) -> f64 {
    let factor = 10f64.powi(digits);
    let scaled = value.abs() * factor;
    // Tolerate floating-point noise such as 51762.50000000001.
    let rounded = if up {
        (scaled - 1e-9).ceil()
    } else {
        (scaled + 1e-9).round()
    };
    value.signum() * rounded / factor
}

/// Excel's serial day number of a `YYYY-MM-DD` date (1 = 1900-01-01).
pub(crate) fn date_serial(iso: &str) -> Option<f64> {
    let bytes = iso.as_bytes();
    if iso.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year: i64 = iso[0..4].parse().ok()?;
    let month: i64 = iso[5..7].parse().ok()?;
    let day: i64 = iso[8..10].parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    // Days from 1970-01-01 (Howard Hinnant's days_from_civil).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    // 1970-01-01 is serial 25569.
    Some((days + 25_569) as f64)
}

#[derive(Default)]
pub(crate) struct Sheet {
    name: String,
    cells: BTreeMap<(u32, u32), Cell>,
    merges: Vec<((u32, u32), (u32, u32))>,
    widths: BTreeMap<u32, f64>,
    /// First unfrozen cell (row, col), like Excel's "Freeze Panes" at that cell.
    freeze: Option<(u32, u32)>,
}

impl Sheet {
    pub(crate) fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub(crate) fn number(&mut self, row: u32, col: u32, value: f64, style: Style) {
        self.put(row, col, Value::Number(value), style);
    }

    /// A non-zero amount; zero leaves the cell empty, as in the workbooks.
    pub(crate) fn amount(&mut self, row: u32, col: u32, value: i64, style: Style) {
        if value != 0 {
            self.number(row, col, value as f64, style);
        } else {
            self.style(row, col, style);
        }
    }

    pub(crate) fn text(&mut self, row: u32, col: u32, value: &str, style: Style) {
        if value.is_empty() {
            self.style(row, col, style);
        } else {
            self.put(row, col, Value::Text(value.to_string()), style);
        }
    }

    /// A `YYYY-MM-DD` date; other text is kept as text.
    pub(crate) fn date(&mut self, row: u32, col: u32, iso: &str) {
        match date_serial(iso) {
            Some(serial) => self.number(row, col, serial, Style::Date),
            None => self.text(row, col, iso, Style::Date),
        }
    }

    /// An empty cell that still carries a style (borders).
    pub(crate) fn style(&mut self, row: u32, col: u32, style: Style) {
        self.cells
            .entry((row, col))
            .and_modify(|cell| cell.style = style)
            .or_insert(Cell {
                value: Value::Text(String::new()),
                style,
            });
    }

    pub(crate) fn formula(&mut self, row: u32, col: u32, expr: &Expr, style: Style) {
        let value = self.eval(expr);
        self.put(row, col, Value::Formula(expr.formula(), value), style);
    }

    /// Writes the formula when it evaluates to `expected` (the app's value),
    /// otherwise `expected` itself, so the sheet never disagrees with the app.
    /// Returns whether the formula was kept.
    pub(crate) fn checked(
        &mut self,
        row: u32,
        col: u32,
        expr: &Expr,
        expected: i64,
        style: Style,
    ) -> bool {
        if (self.eval(expr) - expected as f64).abs() <= 0.5 + 1e-6 {
            self.formula(row, col, expr, style);
            true
        } else {
            self.number(row, col, expected as f64, style);
            false
        }
    }

    pub(crate) fn merge(&mut self, first: (u32, u32), last: (u32, u32)) {
        if first != last {
            self.merges.push((first, last));
        }
    }

    /// A header spanning `first..=last`, with every covered cell bordered.
    pub(crate) fn header(&mut self, first: (u32, u32), last: (u32, u32), label: &str) {
        for row in first.0..=last.0 {
            for col in first.1..=last.1 {
                self.style(row, col, Style::Header);
            }
        }
        self.text(first.0, first.1, label, Style::Header);
        self.merge(first, last);
    }

    pub(crate) fn width(&mut self, col: u32, width: f64) {
        self.widths.insert(col, width);
    }

    pub(crate) fn freeze(&mut self, row: u32, col: u32) {
        self.freeze = Some((row, col));
    }

    /// The numeric value of a cell (0 for empty or text), as Excel reads it in a formula.
    pub(crate) fn value(&self, row: u32, col: u32) -> f64 {
        match self.cells.get(&(row, col)).map(|cell| &cell.value) {
            Some(Value::Number(value)) | Some(Value::Formula(_, value)) => *value,
            _ => 0.0,
        }
    }

    pub(crate) fn eval(&self, expr: &Expr) -> f64 {
        match expr {
            Expr::Cell(row, col) | Expr::Fixed(row, col) => self.value(*row, *col),
            Expr::Num(value) => *value,
            Expr::Add(a, b) => self.eval(a) + self.eval(b),
            Expr::Sub(a, b) => self.eval(a) - self.eval(b),
            Expr::Mul(a, b) => self.eval(a) * self.eval(b),
            Expr::Div(a, b) => {
                let divisor = self.eval(b);
                if divisor == 0.0 {
                    f64::NAN
                } else {
                    self.eval(a) / divisor
                }
            }
            Expr::Sum(first, last) => {
                let mut total = 0.0;
                for row in first.0..=last.0 {
                    for col in first.1..=last.1 {
                        total += self.value(row, col);
                    }
                }
                total
            }
            Expr::Round(a, digits) => round_digits(self.eval(a), *digits, false),
            Expr::RoundUp(a, digits) => round_digits(self.eval(a), *digits, true),
        }
    }

    fn put(&mut self, row: u32, col: u32, value: Value, style: Style) {
        self.cells.insert((row, col), Cell { value, style });
    }

    fn xml(&self) -> String {
        let mut xml = String::from(concat!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" "#,
            r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#
        ));
        xml.push_str(r#"<sheetViews><sheetView workbookViewId="0">"#);
        if let Some((row, col)) = self.freeze {
            let (x, y) = (col - 1, row - 1);
            let pane = match (x > 0, y > 0) {
                (true, true) => "bottomRight",
                (true, false) => "topRight",
                _ => "bottomLeft",
            };
            xml.push_str("<pane");
            if x > 0 {
                xml.push_str(&format!(r#" xSplit="{x}""#));
            }
            if y > 0 {
                xml.push_str(&format!(r#" ySplit="{y}""#));
            }
            xml.push_str(&format!(
                r#" topLeftCell="{}" activePane="{pane}" state="frozen"/><selection pane="{pane}"/>"#,
                cell_ref(row, col)
            ));
        }
        xml.push_str("</sheetView></sheetViews>");
        xml.push_str(r#"<sheetFormatPr defaultRowHeight="15"/>"#);
        if !self.widths.is_empty() {
            xml.push_str("<cols>");
            for (col, width) in &self.widths {
                xml.push_str(&format!(
                    r#"<col min="{col}" max="{col}" width="{width}" customWidth="1"/>"#
                ));
            }
            xml.push_str("</cols>");
        }
        xml.push_str("<sheetData>");
        let mut current_row = 0;
        for (&(row, col), cell) in &self.cells {
            if row != current_row {
                if current_row != 0 {
                    xml.push_str("</row>");
                }
                xml.push_str(&format!(r#"<row r="{row}">"#));
                current_row = row;
            }
            let reference = cell_ref(row, col);
            let style = cell.style as u8;
            match &cell.value {
                Value::Number(value) => xml.push_str(&format!(
                    r#"<c r="{reference}" s="{style}"><v>{}</v></c>"#,
                    format_number(*value)
                )),
                Value::Text(text) if text.is_empty() => {
                    xml.push_str(&format!(r#"<c r="{reference}" s="{style}"/>"#))
                }
                Value::Text(text) => xml.push_str(&format!(
                    r#"<c r="{reference}" s="{style}" t="inlineStr"><is><t xml:space="preserve">{}</t></is></c>"#,
                    escape(text)
                )),
                Value::Formula(formula, value) => {
                    let cached = if value.is_finite() {
                        format!("<v>{}</v>", format_number(*value))
                    } else {
                        String::new()
                    };
                    xml.push_str(&format!(
                        r#"<c r="{reference}" s="{style}"><f>{}</f>{cached}</c>"#,
                        escape(formula)
                    ))
                }
            }
        }
        if current_row != 0 {
            xml.push_str("</row>");
        }
        xml.push_str("</sheetData>");
        if !self.merges.is_empty() {
            xml.push_str(&format!(r#"<mergeCells count="{}">"#, self.merges.len()));
            for (first, last) in &self.merges {
                xml.push_str(&format!(
                    r#"<mergeCell ref="{}:{}"/>"#,
                    cell_ref(first.0, first.1),
                    cell_ref(last.0, last.1)
                ));
            }
            xml.push_str("</mergeCells>");
        }
        xml.push_str(
            r#"<pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>"#,
        );
        xml.push_str("</worksheet>");
        xml
    }

    /// The workbook (.xlsx bytes) holding this sheet.
    pub(crate) fn to_xlsx(&self) -> Result<Vec<u8>, String> {
        let workbook = format!(
            concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" "#,
                r#"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
                r#"<sheets><sheet name="{}" sheetId="1" r:id="rId1"/></sheets>"#,
                r#"<calcPr calcId="191029" fullCalcOnLoad="1"/></workbook>"#
            ),
            escape(&self.name)
        );
        let files: [(&str, &str); 6] = [
            ("[Content_Types].xml", CONTENT_TYPES_XML),
            ("_rels/.rels", ROOT_RELS_XML),
            ("xl/workbook.xml", &workbook),
            ("xl/_rels/workbook.xml.rels", WORKBOOK_RELS_XML),
            ("xl/styles.xml", STYLES_XML),
            ("xl/worksheets/sheet1.xml", &self.xml()),
        ];
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, content) in files {
            zip.start_file(name, options)
                .map_err(|error| error.to_string())?;
            zip.write_all(content.as_bytes())
                .map_err(|error| error.to_string())?;
        }
        zip.finish()
            .map(Cursor::into_inner)
            .map_err(|error| error.to_string())
    }
}

fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            // Control characters are not allowed in XML 1.0.
            ch if (ch as u32) < 0x20 && !matches!(ch, '\t' | '\n' | '\r') => {}
            ch => escaped.push(ch),
        }
    }
    escaped
}

const CONTENT_TYPES_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
    r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
    r#"<Default Extension="xml" ContentType="application/xml"/>"#,
    r#"<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#,
    r#"<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#,
    r#"<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>"#,
    r#"</Types>"#
);

const ROOT_RELS_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>"#,
    r#"</Relationships>"#
);

const WORKBOOK_RELS_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>"#,
    r#"<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>"#,
    r#"</Relationships>"#
);

// Calibri, the header fill (00CCFF) and thin borders of the workbooks; numbers
// as #,##0 with zero shown as "-", dates as dd-mmm-yyyy.
const STYLES_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    r#"<numFmts count="3">"#,
    r##"<numFmt numFmtId="164" formatCode="#,##0;(#,##0);&quot;-&quot;"/>"##,
    r#"<numFmt numFmtId="165" formatCode="dd\-mmm\-yyyy"/>"#,
    r#"<numFmt numFmtId="166" formatCode="0.00%"/>"#,
    r#"</numFmts>"#,
    r#"<fonts count="3">"#,
    r#"<font><sz val="11"/><name val="Calibri"/><family val="2"/></font>"#,
    r#"<font><b/><sz val="11"/><name val="Calibri"/><family val="2"/></font>"#,
    r#"<font><b/><sz val="14"/><name val="Calibri"/><family val="2"/></font>"#,
    r#"</fonts>"#,
    r#"<fills count="3">"#,
    r#"<fill><patternFill patternType="none"/></fill>"#,
    r#"<fill><patternFill patternType="gray125"/></fill>"#,
    r#"<fill><patternFill patternType="solid"><fgColor rgb="FF00CCFF"/><bgColor indexed="64"/></patternFill></fill>"#,
    r#"</fills>"#,
    r#"<borders count="2">"#,
    r#"<border><left/><right/><top/><bottom/><diagonal/></border>"#,
    r#"<border><left style="thin"><color auto="1"/></left><right style="thin"><color auto="1"/></right>"#,
    r#"<top style="thin"><color auto="1"/></top><bottom style="thin"><color auto="1"/></bottom><diagonal/></border>"#,
    r#"</borders>"#,
    r#"<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>"#,
    r#"<cellXfs count="11">"#,
    // 0 Default
    r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>"#,
    // 1 Header
    r#"<xf numFmtId="0" fontId="1" fillId="2" borderId="1" xfId="0" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center" vertical="center" wrapText="1"/></xf>"#,
    // 2 Number
    r#"<xf numFmtId="164" fontId="0" fillId="0" borderId="1" xfId="0" applyNumberFormat="1" applyBorder="1"/>"#,
    // 3 Date
    r#"<xf numFmtId="165" fontId="0" fillId="0" borderId="1" xfId="0" applyNumberFormat="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center"/></xf>"#,
    // 4 Text
    r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="1" xfId="0" applyBorder="1"/>"#,
    // 5 TotalNumber
    r#"<xf numFmtId="164" fontId="1" fillId="0" borderId="1" xfId="0" applyNumberFormat="1" applyFont="1" applyBorder="1"/>"#,
    // 6 TotalText
    r#"<xf numFmtId="0" fontId="1" fillId="0" borderId="1" xfId="0" applyFont="1" applyBorder="1"/>"#,
    // 7 Title
    r#"<xf numFmtId="0" fontId="2" fillId="0" borderId="0" xfId="0" applyFont="1"/>"#,
    // 8 Percent
    r#"<xf numFmtId="166" fontId="0" fillId="0" borderId="1" xfId="0" applyNumberFormat="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center"/></xf>"#,
    // 9 Integer
    r#"<xf numFmtId="1" fontId="0" fillId="0" borderId="1" xfId="0" applyNumberFormat="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center"/></xf>"#,
    // 10 HeaderPercent
    r#"<xf numFmtId="166" fontId="1" fillId="2" borderId="1" xfId="0" applyNumberFormat="1" applyFont="1" applyFill="1" applyBorder="1" applyAlignment="1"><alignment horizontal="center" vertical="center"/></xf>"#,
    r#"</cellXfs>"#,
    r#"<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>"#,
    r#"</styleSheet>"#
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_columns_and_dates_like_excel() {
        assert_eq!(column_name(1), "A");
        assert_eq!(column_name(26), "Z");
        assert_eq!(column_name(27), "AA");
        assert_eq!(column_name(137), "EG");
        assert_eq!(date_serial("1970-01-01"), Some(25_569.0));
        assert_eq!(date_serial("2026-01-02"), Some(46_024.0));
        assert_eq!(date_serial("02-Jan-2026"), None);
    }

    #[test]
    fn keeps_a_formula_only_when_it_gives_the_app_value() {
        let mut sheet = Sheet::new("T");
        sheet.number(3, 7, 1_000.0, Style::Number);
        sheet.number(4, 5, 300.0, Style::Number);
        sheet.number(4, 6, 50.0, Style::Number);
        let balance = Expr::cell(3, 7).sub(Expr::cell(4, 5)).add(Expr::cell(4, 6));
        assert_eq!(balance.formula(), "G3-E4+F4");
        assert!(sheet.checked(4, 7, &balance, 750, Style::Number));
        assert!(!sheet.checked(5, 7, &balance, 700, Style::Number));
        assert_eq!(sheet.value(5, 7), 700.0);
        let nested = Expr::cell(1, 1).sub(Expr::cell(1, 2).add(Expr::cell(1, 3)));
        assert_eq!(nested.formula(), "A1-(B1+C1)");
        let incentive = Expr::num(51_762.5).round_up(-3);
        assert_eq!(sheet.eval(&incentive), 52_000.0);
        assert_eq!(sheet.eval(&Expr::num(2.5).round(0)), 3.0);
    }

    #[test]
    fn writes_a_zip_workbook() {
        let mut sheet = Sheet::new("TAHUN 2026");
        sheet.header((1, 1), (2, 1), "Tanggal");
        sheet.text(3, 2, "A & B <c>", Style::Text);
        sheet.freeze(3, 1);
        let bytes = sheet.to_xlsx().unwrap();
        assert_eq!(&bytes[0..2], b"PK");
        let xml = sheet.xml();
        assert!(xml.contains("A &amp; B &lt;c&gt;"));
        assert!(xml.contains(r#"<mergeCell ref="A1:A2"/>"#));
        assert!(xml.contains(r#"ySplit="2""#));
    }
}
