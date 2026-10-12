//! Exercise workbooks: what the learner sees, and isolated probe copies for grading.

use crate::{LearnError, Result};
use gridcraft_engine::{
    DocState, Session,
    core::{CellRef, Value},
    formula::{FormulaLocale, parse, print_input},
    model::Workbook,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A cell constant or formula. Formulas are stored in canonical (English) syntax.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Cell {
    Num(f64),
    Text(String),
    Formula(String),
    /// Excel serial date, shown as dd.mm.yyyy.
    Date(f64),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Setup {
    /// Cell, content, and whether it is source data (changed by grading probes, must stay intact).
    pub cells: Vec<(String, Cell, bool)>,
    pub bold: Vec<String>,
    pub formats: Vec<(String, String)>,
    pub widths: Vec<(String, f32)>,
    /// Answer cells, lightly highlighted so the learner sees where to type.
    pub answers: Vec<String>,
}

impl Setup {
    pub fn put(&mut self, cell: impl Into<String>, value: Cell, input: bool) {
        let cell = cell.into();
        if let Cell::Date(_) = value {
            self.formats.push((cell.clone(), "dd.mm.yyyy".into()));
        }
        self.cells.push((cell, value, input));
    }
    pub fn text(&mut self, cell: impl Into<String>, t: impl Into<String>) {
        self.put(cell, Cell::Text(t.into()), false);
    }
    pub fn header(&mut self, row: u32, col: u32, titles: &[&str]) {
        for (i, t) in titles.iter().enumerate() {
            self.text(a1(row, col + i as u32), *t);
        }
        if !titles.is_empty() {
            self.bold.push(format!("{}:{}", a1(row, col), a1(row, col + titles.len() as u32 - 1)));
        }
    }
    pub fn input(&mut self, cell: impl Into<String>, value: Cell) {
        self.put(cell, value, true);
    }
    pub fn answer(&mut self, cell: impl Into<String>) {
        self.answers.push(cell.into());
    }
    pub fn width(&mut self, col: u32, width: f32) {
        let c = col_name(col);
        self.widths.push((format!("{c}:{c}"), width));
    }
    pub fn value_of(&self, cell: &str) -> Option<&Cell> {
        self.cells.iter().find(|(c, ..)| c == cell).map(|(_, v, _)| v)
    }

    /// A fresh one-sheet session showing this exercise.
    pub fn session(&self, focus: &str) -> Result<Session> {
        let mut s = Session::new();
        s.new_workbook();
        self.apply(&mut s)?;
        for (range, w) in &self.widths {
            s.execute("home.columnWidth", json!({"cols": range, "width": w}))?;
        }
        for range in &self.bold {
            s.execute("home.bold", json!({"range": range}))?;
        }
        for cell in &self.answers {
            s.execute("home.fillColor", json!({"range": cell, "color": "#FFF4C2"}))?;
        }
        s.execute("selection.set", json!({"range": focus}))?;
        // Setting up the book is not the learner's work.
        if let Ok(d) = s.doc_mut() {
            d.undo.clear();
            d.redo.clear();
        }
        Ok(s)
    }

    /// Writes cells and number formats into the active sheet.
    pub fn apply(&self, s: &mut Session) -> Result<()> {
        for (cell, value, _) in &self.cells {
            set(s, cell, value)?;
        }
        for (range, code) in &self.formats {
            s.execute("home.numberFormat", json!({"range": range, "code": code}))?;
        }
        Ok(())
    }
}

pub fn set(s: &mut Session, cell: &str, value: &Cell) -> Result<()> {
    let params = match value {
        Cell::Num(n) | Cell::Date(n) => json!({"cell": cell, "value": n}),
        Cell::Text(t) => json!({"cell": cell, "value": t}),
        Cell::Formula(f) => json!({"cell": cell, "input": f}),
    };
    s.execute("cell.set", params)?;
    Ok(())
}

pub fn address(s: &str) -> Result<CellRef> {
    CellRef::parse(s).ok_or_else(|| LearnError::Invalid(format!("Неверная ячейка {s}")))
}

pub fn value(s: &Session, cell: &str) -> Result<Value> {
    let d = s.doc()?;
    let sh = d.wb.sheet(d.wb.active_sheet).ok_or_else(|| LearnError::Invalid("Нет листа".into()))?;
    Ok(sh.value(address(cell)?))
}

pub fn has_formula(wb: &Workbook, cell: &str) -> bool {
    let Some(at) = CellRef::parse(cell) else { return false };
    wb.sheet(wb.active_sheet).and_then(|sh| sh.cell(at)).is_some_and(|c| c.formula.is_some())
}

/// Evaluates canonical formulas against `setup` in an isolated book (scratch column ZZ).
pub fn probe(setup: &Setup, formulas: &[&str]) -> Result<Vec<Value>> {
    let mut s = Session::new();
    s.new_workbook();
    setup.apply(&mut s)?;
    let mut out = Vec::new();
    for (i, f) in formulas.iter().enumerate() {
        let cell = format!("ZZ{}", i + 1);
        set(&mut s, &cell, &Cell::Formula((*f).to_string()))?;
        out.push(value(&s, &cell)?);
    }
    Ok(out)
}

/// A copy of the learner's workbook with the source data `old` replaced by `data`.
pub fn swap_data(wb: &Workbook, old: &Setup, data: &Setup) -> Result<Session> {
    let mut s = Session::new();
    s.add_document(DocState::new(wb.clone(), None, "Проверка".into()));
    for (cell, _, input) in &old.cells {
        if *input && !data.cells.iter().any(|(c, _, i)| *i && c == cell) {
            s.execute("edit.clearContents", json!({"range": cell}))?;
        }
    }
    for (cell, value, input) in &data.cells {
        if *input {
            set(&mut s, cell, value)?;
        }
    }
    Ok(s)
}

/// Canonical formula → Russian Excel spelling (`SUMIF(A1:A3,"x")` → `СУММЕСЛИ(A1:A3;"x")`).
pub fn ru(formula: &str) -> String {
    let body = formula.strip_prefix('=').unwrap_or(formula);
    match parse(body).and_then(|e| print_input(&e, FormulaLocale::Ru, |_| false)) {
        Ok(text) => format!("={text}"),
        Err(_) => formula.to_string(),
    }
}

pub fn col_name(col: u32) -> String {
    let mut n = col + 1;
    let mut s = Vec::new();
    while n > 0 {
        let r = ((n - 1) % 26) as u8;
        s.push((b'A' + r) as char);
        n = (n - 1) / 26;
    }
    s.iter().rev().collect()
}

/// 1-based row, 0-based column.
pub fn a1(row: u32, col: u32) -> String {
    format!("{}{row}", col_name(col))
}

pub fn range(col: u32, first: u32, last: u32) -> String {
    format!("{}:{}", a1(first, col), a1(last, col))
}

pub fn abs_range(col: u32, first: u32, last: u32) -> String {
    let c = col_name(col);
    format!("${c}${first}:${c}${last}")
}

/// Excel serial number of a calendar date (1900 date system).
pub fn date(y: i64, m: i64, d: i64) -> f64 {
    // Days from civil, Howard Hinnant's algorithm; 25569 = serial of 1970-01-01.
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146_097 + doe - 719_468 + 25_569) as f64
}

pub fn date_text(serial: f64) -> String {
    let z = serial as i64 - 25_569 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{d:02}.{m:02}.{y}")
}

/// Russian number display: 12 345,5.
pub fn num_text(n: f64) -> String {
    if !n.is_finite() {
        return n.to_string();
    }
    let neg = n < 0.0;
    let r = (n.abs() * 100.0).round() / 100.0;
    let whole = r.trunc() as u64;
    let frac = ((r - r.trunc()) * 100.0).round() as u64;
    let digits = whole.to_string();
    let mut grouped = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(' ');
        }
        grouped.push(ch);
    }
    let mut s = if neg { format!("-{grouped}") } else { grouped };
    if frac > 0 {
        let f = format!("{frac:02}");
        s.push(',');
        s.push_str(f.trim_end_matches('0'));
    }
    s
}

pub fn show(v: &Value) -> String {
    match v {
        Value::Empty => "пусто".into(),
        Value::Number(n) => num_text(*n),
        Value::Text(t) => format!("«{t}»"),
        Value::Bool(b) => if *b { "ИСТИНА" } else { "ЛОЖЬ" }.into(),
        Value::Error(e) => error_ru(e.as_str()).into(),
        Value::Array(_) => "массив".into(),
    }
}

pub fn error_ru(e: &str) -> &'static str {
    match e {
        "#N/A" => "#Н/Д (значение не найдено)",
        "#DIV/0!" => "#ДЕЛ/0! (деление на ноль)",
        "#VALUE!" => "#ЗНАЧ! (не тот тип данных)",
        "#REF!" => "#ССЫЛКА! (ссылка на удалённую ячейку)",
        "#NAME?" => "#ИМЯ? (функция с таким именем не найдена — проверь написание)",
        "#NUM!" => "#ЧИСЛО!",
        "#SPILL!" => "#ПЕРЕНОС!",
        _ => "ошибка",
    }
}
