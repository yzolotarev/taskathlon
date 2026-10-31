//! Deterministic grading. Numbers are computed by GridCraft, never by an LLM.

use crate::sheet::{self, Cell, has_formula, show};
use crate::{Result, Task};
use gridcraft_engine::{
    Session,
    core::{CellRef, RangeRef, Value},
    model::Workbook,
};
use gridcraft_tr::{t as tr, ts};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Kind {
    /// Independent, typed oracle; unlike Formula this does not execute a reference formula.
    Expected {
        cell: String,
        expected: Expectation,
        formula: bool,
    },
    /// Verified against the pivot model, not merely adjacent label/value cells.
    Pivot {
        sheet: String,
        source: String,
        rows: Vec<String>,
        columns: Vec<String>,
        values: Vec<(String, gridcraft_engine::model::PivotFunc)>,
    },
    /// UI state supplied by the host. Missing observation always fails closed.
    Selection {
        range: String,
    },
    /// The cell must match `reference` (canonical syntax) now and on changed source data.
    Formula {
        cell: String,
        reference: String,
        live: bool,
    },
    /// The formula itself is expected to produce a specific error (e.g. a deliberate #N/A).
    FormulaError {
        cell: String,
        reference: String,
        error: String,
        live: bool,
    },
    /// A literal answer (account number, text label).
    Value {
        cell: String,
        expected: Cell,
    },
    Bold {
        range: String,
    },
    Filter,
    FilterShows {
        col: u32,
        first: u32,
        last: u32,
        value: String,
    },
    Sorted {
        first: u32,
        last: u32,
        cols: (u32, u32),
        key: u32,
        descending: bool,
    },
    Frozen {
        rows: u32,
    },
    NumFmt {
        range: String,
        contains: String,
    },
    Unique {
        col: u32,
        first: u32,
        last: u32,
        expected: usize,
    },
    Keys {
        /// Host-reported shortcut evidence only; this does not prove its effect or understanding.
        combo: String,
    },
    /// Label → amount pairs somewhere on the sheet, by any method (pivot, formulas).
    Pairs {
        pairs: Vec<(String, String)>,
    },
    /// A report on the first sheet, by any method: every expected row (found by its key in
    /// column `key`) holds these values, in any row order. `notes` say, per (key, column),
    /// what was wrong with that value in its source, so a miss points back to it.
    Table {
        first: u32,
        left: u32,
        headers: Vec<String>,
        cols: Vec<Col>,
        rows: Vec<Vec<Expectation>>,
        key: usize,
        notes: Vec<(String, usize, String)>,
    },
    /// A work answer from a hand-typed table: an independent oracle, plus the wrong answer
    /// each subset of untreated entry errors produces, so a miss names its trap. With no
    /// `cell` the learner places the answer next to a «Ответ» label at or below row `top`.
    Live {
        cell: Option<String>,
        top: u32,
        expected: f64,
        misses: Vec<(Expectation, String)>,
        /// Cells holding an entry error, with what to say when a formula points at one.
        #[serde(default)]
        dirty: Vec<(String, String)>,
        /// A formula the learner can write to prove the table is clean.
        #[serde(default)]
        control: String,
    },
}

/// How a report column is compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Col {
    /// Same spelling (spaces collapsed, letter case kept): a tidy report uses one name.
    Text,
    /// A real number, not digits stored as text.
    Number,
    /// A real date (serial), not text that looks like one.
    Date,
    /// Given in the template; not graded.
    Given,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Expectation {
    Number(f64),
    Text(String),
    Bool(bool),
    Empty,
    /// Exact engine error, e.g. `#N/A`; useful for teaching what a failed lookup produces.
    Error(String),
}

impl Expectation {
    fn matches(&self, value: &Value) -> bool {
        match (self, value) {
            (Self::Number(n), Value::Number(v)) => equivalent(&Value::number(*n), &Value::number(*v)),
            (Self::Text(t), Value::Text(v)) => fold(t) == fold(v),
            (Self::Bool(b), Value::Bool(v)) => b == v,
            (Self::Empty, Value::Empty) => true,
            (Self::Error(e), Value::Error(v)) => e == v.as_str(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Observation {
    /// Exact current selection, optionally sheet-qualified. Not a keypress inference.
    pub selection: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct SourcePolicy {
    /// Source cells intentionally editable by this task; all other inputs remain protected.
    pub editable: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Check {
    pub label: String,
    pub skill: String,
    pub kind: Kind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Outcome {
    pub label: String,
    pub skill: String,
    pub passed: bool,
    pub feedback: String,
    /// What a correct answer looks like, shown after a miss.
    pub solution: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evaluation {
    pub score: f32,
    pub outcomes: Vec<Outcome>,
}

impl Evaluation {
    pub fn passed(&self) -> bool {
        !self.outcomes.is_empty() && self.outcomes.iter().all(|o| o.passed)
    }
}

pub fn equivalent(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.is_finite() && y.is_finite() && (x - y).abs() <= 0.005_f64.max(1e-9 * y.abs()),
        (Value::Text(x), Value::Text(y)) => fold(x) == fold(y),
        (Value::Error(x), Value::Error(y)) => x == y,
        (Value::Empty, Value::Text(t)) | (Value::Text(t), Value::Empty) => t.trim().is_empty(),
        _ => a == b,
    }
}

fn fold(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase().replace('ё', "е")
}

fn cell_value(c: &Cell) -> Value {
    match c {
        Cell::Num(n) | Cell::Date(n) => Value::number(*n),
        Cell::Text(t) => Value::text(t.as_str()),
        Cell::Formula(_) => Value::Empty,
    }
}

fn sheet_of(wb: &Workbook) -> Option<&gridcraft_engine::model::Sheet> {
    wb.sheet(wb.active_sheet)
}

fn at(wb: &Workbook, row: u32, col: u32) -> Value {
    sheet_of(wb).map(|sh| sh.value(CellRef::new(row.saturating_sub(1), col))).unwrap_or_default()
}

fn intact(task: &Task, wb: &Workbook, policy: &SourcePolicy) -> bool {
    first_change(task, wb, policy).is_none()
}

/// The first source cell that no longer holds what the task gave: (cell, was, now).
fn first_change(task: &Task, wb: &Workbook, policy: &SourcePolicy) -> Option<(String, String, String)> {
    task.setup.cells.iter().filter(|(_, _, input)| *input).find_map(|(cell, v, _)| {
        let same = source_kept(wb, policy, cell, v);
        (!same).then(|| {
            let now = sheet::resolve_cell(wb, cell)
                .ok()
                .and_then(|(index, a)| wb.sheet(index).and_then(|sh| sh.cell(a).map(|c| c.input_text())))
                .unwrap_or_default();
            (cell.clone(), show(&cell_value(v)), now)
        })
    })
}

/// What a learner sees when the source was edited: which cell, and how to get it back.
fn source_changed(change: &(String, String, String)) -> String {
    let (cell, was, now) = change;
    if now.is_empty() {
        tr(&format!(
            "Ты стёр исходную ячейку {cell} (там было «{was}»). Исходные данные в этом задании не меняют: верни (Ctrl+Z или «Начать заново»)"
        ))
    } else {
        tr(&format!(
            "Ты изменил исходную ячейку {cell}: было «{was}», стало «{now}». В этом задании данные не исправляют: верни как было (Ctrl+Z или «Начать заново»)"
        ))
    }
}

fn source_kept(wb: &Workbook, policy: &SourcePolicy, cell: &str, v: &Cell) -> bool {
    {
        if policy
            .editable
            .iter()
            .any(|allowed| sheet::resolve_cell(wb, allowed).ok() == sheet::resolve_cell(wb, cell).ok() && sheet::resolve_cell(wb, cell).is_ok())
        {
            return true;
        }
        sheet::resolve_cell(wb, cell).is_ok_and(|(index, a)| {
            let sh = wb.sheet(index);
            sh.is_some_and(|sh| sh.cell(a).is_none_or(|c| c.formula.is_none()) && equivalent(&sh.value(a), &cell_value(v)))
        })
    }
}

pub fn evaluate(task: &Task, s: &Session, pressed: &[String]) -> Result<Evaluation> {
    evaluate_with_policy_and_observation(task, s, pressed, &SourcePolicy::default(), &Observation::default())
}

pub fn evaluate_with_policy(task: &Task, s: &Session, pressed: &[String], policy: &SourcePolicy) -> Result<Evaluation> {
    evaluate_with_policy_and_observation(task, s, pressed, policy, &Observation::default())
}

pub fn evaluate_with_observation(task: &Task, s: &Session, pressed: &[String], observation: &Observation) -> Result<Evaluation> {
    evaluate_with_policy_and_observation(task, s, pressed, &SourcePolicy::default(), observation)
}

pub fn evaluate_with_policy_and_observation(
    task: &Task,
    s: &Session,
    pressed: &[String],
    policy: &SourcePolicy,
    observation: &Observation,
) -> Result<Evaluation> {
    let mut wb = s.doc()?.wb.as_ref().clone();
    // Bare task addresses refer to the original exercise sheet, not the UI's current tab. A
    // pivot inserted on a new sheet goes in front of it, so find it by name, not position.
    wb.active_sheet = wb.sheet_index(&home_sheet()).unwrap_or(0);
    let source_ok = task.reorders || intact(task, &wb, policy);
    let mut outcomes = Vec::new();
    for c in &task.checks {
        match &c.kind {
            Kind::FilterShows { first, last, col, .. } | Kind::Unique { first, last, col, .. }
                if *first == 0
                    || first > last
                    || *last > gridcraft_engine::core::MAX_ROWS
                    || *col >= gridcraft_engine::core::MAX_COLS
                    || last - first >= 50_000 =>
            {
                return Err(crate::LearnError::Invalid("Invalid check bounds".into()));
            }
            Kind::Sorted { first, last, cols, key, .. }
                if *first == 0
                    || first > last
                    || *last > gridcraft_engine::core::MAX_ROWS
                    || cols.0 > cols.1
                    || cols.1 >= gridcraft_engine::core::MAX_COLS
                    || *key < cols.0
                    || *key > cols.1
                    || u64::from(last - first + 1) * u64::from(cols.1 - cols.0 + 1) > 50_000 =>
            {
                return Err(crate::LearnError::Invalid("Invalid check bounds".into()));
            }
            _ => {}
        }
        let (passed, feedback, solution) = match &c.kind {
            Kind::Expected { cell, expected, formula } => {
                let ok = source_ok && expected.matches(&sheet::workbook_value(&wb, cell)?) && (!formula || has_formula(&wb, cell));
                (ok, if ok { ts("Верно").into() } else { ts("Проверь ответ и исходные данные").into() }, None)
            }
            Kind::Selection { range } => {
                let parse = |text: &str| {
                    let (prefix, body) = text.rsplit_once('!').map_or((None, text), |(p, b)| (Some(p), b));
                    let r = RangeRef::parse(body)?;
                    let (index, _) = sheet::resolve_cell(&wb, &prefix.map_or_else(|| r.start.a1(), |p| format!("{p}!{}", r.start.a1()))).ok()?;
                    Some((index, r))
                };
                let want = parse(range);
                let ok = want.is_some() && observation.selection.as_deref().and_then(parse) == want;
                (ok, if ok { ts("Верно").into() } else { tr(&format!("Выдели {range}")) }, Some(range.clone()))
            }
            Kind::Pivot { sheet, source, rows, columns, values } => {
                let (ok, why) = pivot_check(&wb, sheet, source, rows, columns, values);
                (ok && source_ok, why, None)
            }
            Kind::Formula { cell, reference, live } => formula(task, &wb, cell, reference, *live, source_ok)?,
            Kind::FormulaError { cell, reference, error, live } => formula_error(task, &wb, cell, reference, error, *live, source_ok)?,
            Kind::Value { cell, expected } => {
                let actual = sheet::workbook_value(&wb, cell)?;
                let want = cell_value(expected);
                let ok = equivalent(&actual, &want)
                    || matches!((expected, &actual), (Cell::Num(n), Value::Text(t)) if t.trim().parse::<f64>().is_ok_and(|x| (x - n).abs() < 1e-9));
                let fb = if ok {
                    ts("Верно").into()
                } else if matches!(actual, Value::Empty) {
                    tr(&format!("{cell} пока пустая"))
                } else {
                    tr(&format!("В {cell} стоит {}, а нужно {}", show(&actual), show(&want)))
                };
                (ok, fb, Some(show(&want)))
            }
            Kind::Bold { range } => {
                let ok = sheet::resolve_range(&wb, range)
                    .is_ok_and(|(index, r)| all_cells(&r.a1(), |a| wb.sheet(index).is_some_and(|sh| wb.styles.get(sh.style_id(a)).font.bold)));
                (ok, if ok { ts("Заголовки жирные").into() } else { tr(&format!("{range} ещё не жирный")) }, Some("Ctrl+B".into()))
            }
            Kind::Filter => {
                let ok = sheet_of(&wb).is_some_and(|sh| sh.autofilter.is_some());
                (
                    ok,
                    if ok { ts("Фильтр включён").into() } else { ts("Фильтр не включён").into() },
                    Some(ts("Данные → Фильтр или Ctrl+Shift+L").into()),
                )
            }
            Kind::FilterShows { col, first, last, value } => {
                let sh = sheet_of(&wb);
                let has_filter = sh.is_some_and(|sh| sh.autofilter.is_some());
                let mut ok = has_filter;
                for r in *first..=*last {
                    let hidden = sh.is_some_and(|sh| sh.rows.get(&(r - 1)).is_some_and(|l| l.hidden));
                    let matches = at(&wb, r, *col).as_text().is_some_and(|t| fold(t) == fold(value));
                    if hidden == matches {
                        ok = false;
                    }
                }
                let fb = if ok {
                    tr(&format!("Видны только строки «{value}»"))
                } else if !has_filter {
                    ts("Сначала включи фильтр").into()
                } else {
                    tr(&format!("Должны остаться видны только строки «{value}»"))
                };
                (ok, fb, Some(tr(&format!("Стрелка фильтра в заголовке → оставить «{value}»"))))
            }
            Kind::Sorted { first, last, cols, key, descending } => {
                let row = |r: u32| (cols.0..=cols.1).map(|c| at(&wb, r, c)).collect::<Vec<_>>();
                let rows: Vec<Vec<Value>> = (*first..=*last).map(row).collect();
                let key_i = key.saturating_sub(cols.0) as usize;
                let ordered = rows.windows(2).all(|w| match (w.first().and_then(|r| r.get(key_i)), w.get(1).and_then(|r| r.get(key_i))) {
                    (Some(a), Some(b)) => {
                        let o = compare(a, b);
                        if *descending { o != std::cmp::Ordering::Less } else { o != std::cmp::Ordering::Greater }
                    }
                    _ => true,
                });
                let mut before: Vec<String> = original_rows(task, *first, *last, *cols);
                let mut after: Vec<String> = rows.iter().map(|r| row_key(r)).collect();
                before.sort();
                after.sort();
                let together = before == after;
                let fb = if ordered && together {
                    ts("Отсортировано").into()
                } else if !together {
                    ts("Строки перемешались: сортируй всю таблицу целиком, а не один столбец").into()
                } else {
                    tr(&format!("Порядок {} ещё не тот", if *descending { "по убыванию" } else { "по возрастанию" }))
                };
                (ordered && together, fb, Some(ts("Данные → Сортировка").into()))
            }
            Kind::Frozen { rows } => {
                let ok = sheet_of(&wb).and_then(|sh| sh.freeze).is_some_and(|(r, _)| r == *rows);
                (ok, if ok { ts("Шапка закреплена").into() } else { ts("Шапка не закреплена").into() }, Some(ts("Вид → Закрепить области").into()))
            }
            Kind::NumFmt { range, contains } => {
                let ok = sheet::resolve_range(&wb, range).is_ok_and(|(index, r)| {
                    all_cells(&r.a1(), |a| {
                        wb.sheet(index).is_some_and(|sh| wb.styles.get(sh.style_id(a)).num_fmt.as_str().contains(contains.as_str()))
                    })
                });
                (
                    ok,
                    if ok { ts("Формат применён").into() } else { tr(&format!("У {range} другой формат чисел")) },
                    Some(ts("Ctrl+1 → Числовой, разделитель групп").into()),
                )
            }
            Kind::Unique { col, first, last, expected } => {
                let mut seen = Vec::new();
                let mut dupes = false;
                for r in *first..=*last {
                    let v = at(&wb, r, *col);
                    if matches!(v, Value::Empty) {
                        continue;
                    }
                    let k = show(&v);
                    if seen.contains(&k) {
                        dupes = true;
                    }
                    seen.push(k);
                }
                let ok = !dupes && seen.len() == *expected;
                (
                    ok,
                    if ok {
                        ts("Дубликаты удалены").into()
                    } else {
                        tr(&format!("Должно остаться {expected} уникальных строк"))
                    },
                    Some(ts("Данные → Удалить дубликаты").into()),
                )
            }
            Kind::Keys { combo } => {
                let ok = pressed.iter().any(|p| p == combo);
                (ok, if ok { tr(&format!("{combo} — есть!")) } else { tr(&format!("Сделай это клавишами {combo}")) }, Some(combo.clone()))
            }
            Kind::Pairs { pairs } => pairs_check(task, &wb, pairs)?,
            Kind::Live { cell, top, expected, misses, dirty, control } => {
                live_check(task, &wb, cell.as_deref(), *top, *expected, misses, dirty, control)
            }
            Kind::Table { first, left, headers, cols, rows, key, notes } => {
                // A pivot inserted onto a new sheet lands in front: the report is found by name.
                let report = task.setup.first_sheet.as_deref().and_then(|n| wb.sheet_index(n)).unwrap_or(0);
                table_check(&wb, report, *first, *left, headers, cols, rows, *key, notes)
            }
        };
        let protected = !matches!(c.kind, Kind::Keys { .. } | Kind::Selection { .. });
        let passed = passed && (!protected || source_ok);
        let feedback = if protected && !source_ok {
            first_change(task, &wb, policy)
                .map(|c| source_changed(&c))
                .unwrap_or_else(|| ts("Исходные данные изменены — их трогать нельзя. Отмени правку (Ctrl+Z)").into())
        } else {
            feedback
        };
        outcomes.push(Outcome { label: c.label.clone(), skill: c.skill.clone(), passed, feedback, solution });
    }
    let passed = outcomes.iter().filter(|o| o.passed).count();
    Ok(Evaluation { score: passed as f32 / outcomes.len().max(1) as f32, outcomes })
}

fn compare(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Text(x), Value::Text(y)) => fold(x).cmp(&fold(y)),
        _ => std::cmp::Ordering::Equal,
    }
}

fn original_rows(task: &Task, first: u32, last: u32, cols: (u32, u32)) -> Vec<String> {
    (first..=last)
        .map(|r| row_key(&(cols.0..=cols.1).map(|c| task.setup.value_of(&sheet::a1(r, c)).map(cell_value).unwrap_or_default()).collect::<Vec<_>>()))
        .collect()
}

fn row_key(values: &[Value]) -> String {
    // Typed, escaped serialization avoids collisions from delimiters or rounded display numbers.
    serde_json::to_string(values).unwrap_or_default()
}

fn all_cells(range: &str, f: impl Fn(CellRef) -> bool) -> bool {
    let Some(r) = RangeRef::parse(range).or_else(|| CellRef::parse(range).map(|c| RangeRef::new(c, c))) else { return false };
    if r.count() > 50_000 {
        return false;
    }
    (r.start.row..=r.end.row).all(|row| (r.start.col..=r.end.col).all(|col| f(CellRef::new(row, col))))
}

fn formula(task: &Task, wb: &Workbook, cell: &str, reference: &str, live: bool, source_ok: bool) -> Result<(bool, String, Option<String>)> {
    // In the staged VLOOKUP→IFERROR ladder, revealing the complete nested formula on
    // the first miss skips the very composition we are trying to teach. Hints/scaffolding
    // are supplied by the lesson instead; the full solution remains visible in other drills.
    let staged_lookup = matches!(task.spec.skill.as_str(), "lookup.error_observe" | "lookup.iferror.wrap" | "lookup.iferror.combine");
    let solution = (!staged_lookup).then(|| sheet::ru(reference));
    let actual = sheet::workbook_value(wb, cell)?;
    let expected = sheet::probe(&task.setup, &[reference])?.into_iter().next().unwrap_or_default();
    let is_formula = has_formula(wb, cell);
    if !source_ok {
        return Ok((false, ts("Исходные данные изменены — их трогать нельзя. Отмени правку (Ctrl+Z)").into(), solution));
    }
    if matches!(actual, Value::Empty) {
        return Ok((false, tr(&format!("{cell} пока пустая")), solution));
    }
    let wrong = matches!(actual, Value::Error(_)) || !equivalent(&actual, &expected);
    if wrong && let Some(why) = reference_slip(wb, cell, reference) {
        return Ok((false, why, solution));
    }
    if let Value::Error(e) = &actual {
        return Ok((false, tr(&format!("Формула вернула {}", sheet::error_ru(e.as_str()))), solution));
    }
    if let Some(why) = text_number(cell, &actual, &expected) {
        return Ok((false, why, solution));
    }
    if !equivalent(&actual, &expected) {
        return Ok((false, tr(&format!("В {cell} получилось {}, а должно быть {}", show(&actual), show(&expected))), solution));
    }
    if live && !is_formula {
        return Ok((false, ts("Ответ верный, но вписан вручную. Нужна формула — она пересчитается на новых данных").into(), solution));
    }
    if live {
        // Same layout, different numbers: a real formula follows, a typed result does not.
        for p in 1..=2u64 {
            let mut spec = task.spec.clone();
            spec.data = spec.data.wrapping_add(p.wrapping_mul(7919));
            let Ok(other) = crate::content::build(&spec) else { continue };
            let probe = sheet::swap_data(wb, &task.setup, &other.setup)?;
            let got = sheet::value(&probe, cell)?;
            let want = sheet::probe(&other.setup, &[reference])?.into_iter().next().unwrap_or_default();
            if !equivalent(&got, &want) {
                if let Some(why) = text_number(cell, &got, &want) {
                    return Ok((false, why, solution));
                }
                return Ok((false, ts("На других данных формула ошибается. Проверь диапазоны и закрепление $").into(), solution));
            }
        }
    }
    Ok((true, ts("Верно!").into(), solution))
}

fn formula_error(
    task: &Task,
    wb: &Workbook,
    cell: &str,
    reference: &str,
    error: &str,
    live: bool,
    source_ok: bool,
) -> Result<(bool, String, Option<String>)> {
    let actual = sheet::workbook_value(wb, cell)?;
    let expected = sheet::probe(&task.setup, &[reference])?.into_iter().next().unwrap_or_default();
    let formula_present = has_formula(wb, cell);
    let matches_error = matches!(&actual, Value::Error(e) if e.as_str() == error) && matches!(&expected, Value::Error(e) if e.as_str() == error);
    if !source_ok {
        return Ok((false, ts("Исходные данные изменены — их трогать нельзя. Отмени правку (Ctrl+Z)").into(), Some(reference.into())));
    }
    if !formula_present || !matches_error {
        if let Some(why) = reference_slip(wb, cell, reference) {
            return Ok((false, why, None));
        }
        return Ok((false, tr(&format!("В {cell} ожидается формула, которая показывает {error}")), None));
    }
    if live {
        for p in 1..=2u64 {
            let mut spec = task.spec.clone();
            spec.data = spec.data.wrapping_add(p.wrapping_mul(7919));
            let other = crate::content::build(&spec)?;
            let probe = sheet::swap_data(wb, &task.setup, &other.setup)?;
            let got = sheet::value(&probe, cell)?;
            let want = sheet::probe(&other.setup, &[reference])?.into_iter().next().unwrap_or_default();
            if !matches!((&got, &want), (Value::Error(a), Value::Error(b)) if a.as_str() == error && b.as_str() == error) {
                return Ok((false, tr(&format!("На изменённых данных {cell} больше не показывает {error}")), None));
            }
        }
    }
    Ok((true, tr(&format!("В {cell} видно {error}; теперь сравни с найденным кодом.")), None))
}

/// A number written as text («"0"» in quotes): looks right, but sums and comparisons skip it.
fn text_number(cell: &str, actual: &Value, expected: &Value) -> Option<String> {
    let (Value::Text(t), Value::Number(n)) = (actual, expected) else { return None };
    let typed: f64 = t.trim().replace(',', ".").parse().ok()?;
    ((typed - n).abs() < 1e-9).then(|| {
        tr(&format!(
            "В {cell} число {} записано как текст: в кавычках \"{}\" это текст. Пиши число без кавычек, иначе СУММ и сравнения его пропустят",
            t.trim(),
            t.trim()
        ))
    })
}

/// The learner's formula is the reference except for one cell address: a slipped reference,
/// such as the order code of the neighbouring row. Names the typed and the needed address.
fn reference_slip(wb: &Workbook, cell: &str, reference: &str) -> Option<String> {
    use gridcraft_engine::formula::lexer::{Tok, tokenize};
    let (index, at) = sheet::resolve_cell(wb, cell).ok()?;
    let typed = wb.sheet(index)?.cell(at)?.formula.as_ref()?.text.clone();
    let toks = |f: &str| -> Option<Vec<Tok>> {
        Some(tokenize(f.trim_start_matches('=')).ok()?.into_iter().map(|t| t.tok).filter(|t| !matches!(t, Tok::Space | Tok::Eof)).collect())
    };
    let (a, b) = (toks(&typed)?, toks(reference)?);
    if a.len() != b.len() {
        return None;
    }
    let norm = |w: &str| w.replace('$', "").to_uppercase();
    let mut slip = None;
    for (x, y) in a.iter().zip(&b) {
        match (x, y) {
            (Tok::Word(p), Tok::Word(q)) if norm(p) == norm(q) => {}
            (Tok::Word(p), Tok::Word(q)) if slip.is_none() && CellRef::parse(&norm(p)).is_some() && CellRef::parse(&norm(q)).is_some() => {
                slip = Some((p.clone(), q.clone()));
            }
            (Tok::Func(p), Tok::Func(q)) if p.eq_ignore_ascii_case(q) => {}
            _ if x == y => {}
            _ => return None,
        }
    }
    let (got, want) = slip?;
    Some(tr(&format!("Формула в {cell} берёт {got}, а здесь нужна {want}: ссылка указывает не на ту ячейку.")))
}

/// Name of the sheet every exercise starts on.
pub fn home_sheet() -> String {
    static HOME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    HOME.get_or_init(|| {
        let mut s = Session::new();
        s.new_workbook();
        s.doc().ok().and_then(|d| d.wb.sheet(0).map(|sh| sh.name.clone())).unwrap_or_else(|| "Sheet1".into())
    })
    .clone()
}

/// A real pivot object with exactly these fields. An empty `sheet` accepts any sheet (Insert →
/// PivotTable puts it on a new one). The message names the first thing that differs.
fn pivot_check(
    wb: &Workbook,
    sheet: &str,
    source: &str,
    rows: &[String],
    columns: &[String],
    values: &[(String, gridcraft_engine::model::PivotFunc)],
) -> (bool, String) {
    use gridcraft_engine::model::{PivotFunc, PivotShowAs};
    let pivots: Vec<&gridcraft_engine::model::PivotTable> = wb
        .sheets
        .iter()
        .filter(|sh| sheet.is_empty() || sh.name == sheet)
        .flat_map(|sh| sh.pivots.iter())
        .filter(|p| p.last_range.is_some())
        .collect();
    let Some(last) = pivots.last() else {
        return (false, ts("Сводной таблицы нет. Встань в таблицу → Вставка → Сводная таблица → поля справа").into());
    };
    let names = |list: &[String]| if list.is_empty() { ts("ничего").to_string() } else { list.join(", ") };
    let fields = |f: &[gridcraft_engine::model::PivotField]| f.iter().map(|x| x.source_col.clone()).collect::<Vec<_>>();
    let same_source = |p: &gridcraft_engine::model::PivotTable| {
        sheet::resolve_range(wb, &p.source).ok().is_some_and(|actual| sheet::resolve_range(wb, source).is_ok_and(|expected| actual == expected))
    };
    let exact = |p: &gridcraft_engine::model::PivotTable| {
        same_source(p)
            && fields(&p.rows) == rows
            && fields(&p.columns) == columns
            && p.values.iter().map(|v| (&v.source_col, v.func)).eq(values.iter().map(|(name, func)| (name, *func)))
            && p.filters.is_empty()
            && p.calculated_fields.is_empty()
            && p.values.iter().all(|v| v.show_as == PivotShowAs::Normal)
    };
    if pivots.iter().any(|p| exact(p)) {
        return (true, ts("Верно! Сводная собрана правильно").into());
    }
    let en = gridcraft_tr::english();
    let func_name = |f: PivotFunc| match (f, en) {
        (PivotFunc::Sum, false) => "Сумма",
        (PivotFunc::Sum, true) => "Sum",
        (PivotFunc::Count | PivotFunc::CountNumbers, false) => "Количество",
        (PivotFunc::Count | PivotFunc::CountNumbers, true) => "Count",
        (PivotFunc::Average, false) => "Среднее",
        (PivotFunc::Average, true) => "Average",
        (PivotFunc::Max, false) => "Максимум",
        (PivotFunc::Max, true) => "Max",
        (PivotFunc::Min, false) => "Минимум",
        (PivotFunc::Min, true) => "Min",
        (_, false) => "другая функция",
        (_, true) => "another function",
    };
    let why = if !same_source(last) {
        tr(&format!("Сводная построена не по всей таблице ({}). Встань в таблицу и создай её заново", last.source))
    } else if fields(&last.rows) != rows {
        tr(&format!("В «Строках» сейчас: {}, а нужно: {}", names(&fields(&last.rows)), names(rows)))
    } else if fields(&last.columns) != columns {
        tr(&format!("В «Столбцах» сейчас: {}, а нужно: {}", names(&fields(&last.columns)), names(columns)))
    } else if let Some(((field, want), v)) = values.first().zip(last.values.first())
        && v.source_col == *field
        && v.func != *want
    {
        tr(&format!(
            "В «Значениях» считается «{}», а нужно «{}»: меню поля в «Значениях» → Итоги по: {}",
            func_name(v.func),
            func_name(*want),
            func_name(*want)
        ))
    } else if last.values.iter().map(|v| &v.source_col).ne(values.iter().map(|(name, _)| name)) {
        let have: Vec<String> = last.values.iter().map(|v| v.source_col.clone()).collect();
        let want: Vec<String> = values.iter().map(|(n, _)| n.clone()).collect();
        tr(&format!("В «Значениях» сейчас: {}, а нужно: {}", names(&have), names(&want)))
    } else {
        ts("Убери из сводной лишнее: фильтры, вычисляемые поля и дополнительные вычисления").into()
    };
    (false, why)
}

fn pairs_check(task: &Task, wb: &Workbook, pairs: &[(String, String)]) -> Result<(bool, String, Option<String>)> {
    let refs: Vec<&str> = pairs.iter().map(|(_, f)| f.as_str()).collect();
    let expected = sheet::probe(&task.setup, &refs)?;
    // Labels in the source data or in the case's answer cells don't count as a summary.
    let inputs: Vec<CellRef> =
        task.setup.cells.iter().filter(|(_, _, i)| *i).map(|(c, ..)| c).chain(task.setup.answers.iter()).filter_map(|c| CellRef::parse(c)).collect();
    if wb.sheets.is_empty() {
        return Ok((false, ts("Нет листа").into(), None));
    }
    let mut missing = Vec::new();
    for ((label, _), want) in pairs.iter().zip(expected.iter()) {
        let mut found = false;
        for (index, sh) in wb.sheets.iter().enumerate() {
            for (a, c) in sh.cells.iter() {
                if (index == 0 && inputs.contains(&a)) || !c.value.as_text().is_some_and(|t| fold(t) == fold(label)) {
                    continue;
                }
                for dc in 1..=8u32 {
                    if a.offset(0, i64::from(dc)).is_some_and(|next| equivalent(&sh.value(next), want)) {
                        found = true;
                    }
                }
            }
        }
        if !found {
            missing.push(format!("{label}: {}", show(want)));
        }
    }
    let solution = Some(pairs.iter().zip(expected.iter()).map(|((l, _), v)| format!("{l} — {}", show(v))).collect::<Vec<_>>().join("; "));
    if missing.is_empty() {
        Ok((true, ts("Сводка сходится").into(), solution))
    } else {
        Ok((false, tr(&format!("Не нашёл в сводке: {}", missing.join("; "))), solution))
    }
}

/// Where a self-placed live answer is: right of (or just below) a «Ответ» label.
/// Where a self-placed live answer sits: right of (or under) the learner's «Ответ» label.
pub fn live_answer_cell(task: &Task, wb: &Workbook, top: u32) -> Option<CellRef> {
    let sh = wb.sheet(0)?;
    let given: Vec<CellRef> = task.setup.cells.iter().filter_map(|(c, ..)| CellRef::parse(c)).collect();
    let label = fold(crate::content::live::ANSWER_LABEL);
    let mut labels: Vec<CellRef> = sh
        .cells
        .iter()
        .filter(|(a, c)| {
            a.row + 1 >= top && !given.contains(a) && c.value.as_text().is_some_and(|t| fold(t.trim().trim_end_matches(':').trim()) == label)
        })
        .map(|(a, _)| a)
        .collect();
    labels.sort_by_key(|a| (a.row, a.col));
    labels.into_iter().find_map(|a| {
        let right = (1..=8i64).filter_map(|dc| a.offset(0, dc));
        let below = (1..=2i64).filter_map(|dr| a.offset(dr, 0));
        right.chain(below).find(|at| sh.cell(*at).is_some() && !matches!(sh.value(*at), Value::Empty))
    })
}

/// A formula that only restates a number (`=143500`) is a typed answer, not a calculation.
fn references_cells(text: &str) -> bool {
    let b = text.as_bytes();
    b.windows(2).any(|w| w.first().is_some_and(u8::is_ascii_alphabetic) && w.get(1).is_some_and(u8::is_ascii_digit))
}

#[allow(clippy::too_many_arguments)]
fn live_check(
    task: &Task,
    wb: &Workbook,
    cell: Option<&str>,
    top: u32,
    expected: f64,
    misses: &[(Expectation, String)],
    dirty: &[(String, String)],
    control: &str,
) -> (bool, String, Option<String>) {
    let label = crate::content::live::ANSWER_LABEL;
    let solution = Some(sheet::num_text(expected));
    let at = match cell {
        Some(c) => CellRef::parse(c),
        None => live_answer_cell(task, wb, top),
    };
    let Some(at) = at else {
        return (false, tr(&format!("Не нашёл ответ: напиши в ячейке «{label}», а справа от неё — формулу")), solution);
    };
    let name = at.a1();
    let Some(sh) = wb.sheet(0) else { return (false, ts("Нет листа").into(), solution) };
    let actual = sh.value(at);
    let formula = sh.cell(at).and_then(|c| c.formula.as_ref().map(|f| f.text.to_string()));
    if matches!(actual, Value::Empty) {
        return (false, tr(&format!("{name} пока пустая")), solution);
    }
    let want = Value::number(expected);
    if equivalent(&actual, &want) {
        return match formula {
            Some(f) if references_cells(&f) => (true, ts("Верно! Данные проверены и посчитаны").into(), solution),
            _ => (false, ts("Число верное, но вписано вручную. Нужна формула от таблицы — она пересчитается, когда реестр обновят").into(), solution),
        };
    }
    let got = match &actual {
        Value::Error(e) => tr(&format!("Формула вернула {}", sheet::error_ru(e.as_str()))),
        other => tr(&format!("В {name} получилось {}, а должно быть {}", show(other), show(&want))),
    };
    // A formula that takes its condition or value from a broken cell repeats the entry error.
    let pointed = formula.as_deref().map(single_cells).unwrap_or_default();
    if let Some((_, why)) = dirty.iter().find(|(at, _)| pointed.iter().any(|p| p == at)) {
        return (false, tr(&format!("{got}. {why}")), solution);
    }
    match misses.iter().find(|(e, _)| e.matches(&actual)) {
        Some((_, why)) => (false, tr(&format!("{got}. Похоже, не учтено: {why}")), solution),
        None if !control.is_empty() => (false, tr(&format!("{got}. В таблице есть ошибки ввода. Проверь себя формулой: {control}")), solution),
        None => (false, tr(&format!("{got}. В таблице есть ошибки ввода — сверь вручную 2–3 строки, которые попадают в расчёт")), solution),
    }
}

/// Single-cell references of a canonical formula (range ends such as `A3` in `A3:A12` excluded),
/// without `$`.
fn single_cells(formula: &str) -> Vec<String> {
    use gridcraft_engine::formula::lexer::{Tok, tokenize};
    let Ok(toks) = tokenize(formula.trim_start_matches('=')) else { return Vec::new() };
    let toks: Vec<Tok> = toks.into_iter().map(|t| t.tok).filter(|t| !matches!(t, Tok::Space | Tok::Eof)).collect();
    let mut out = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        let Tok::Word(w) = t else { continue };
        let in_range = matches!(toks.get(i + 1), Some(Tok::Colon)) || i > 0 && matches!(toks.get(i - 1), Some(Tok::Colon));
        let a = w.replace('$', "").to_uppercase();
        if !in_range && CellRef::parse(&a).is_some() {
            out.push(a);
        }
    }
    out
}

fn tidy(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Day and month swapped: what a US-style date reading does to 05.03 vs 03.05.
fn swapped(serial: f64) -> Option<f64> {
    let (y, m, d) = crate::sheet::ymd(serial)?;
    (d <= 12 && d != m).then(|| crate::sheet::date(y, d, m))
}

#[allow(clippy::too_many_arguments)]
fn table_check(
    wb: &Workbook,
    report: usize,
    first: u32,
    left: u32,
    headers: &[String],
    cols: &[Col],
    rows: &[Vec<Expectation>],
    key: usize,
    notes: &[(String, usize, String)],
) -> (bool, String, Option<String>) {
    let Some(sh) = wb.sheet(report) else { return (false, ts("Нет листа").into(), None) };
    let at = |row: u32, col: usize| sh.value(CellRef::new(row.saturating_sub(1), left + col as u32));
    let key_text = |v: &Value| match v {
        Value::Text(t) => fold(t),
        Value::Number(n) => show(&Value::Number(*n)),
        _ => String::new(),
    };
    // The report rows as they stand: empty and «Итого» rows are not data.
    let mut actual: Vec<(u32, String)> = Vec::new();
    let mut blank_run = 0;
    let mut r = first;
    while blank_run < 5 && r < first + rows.len() as u32 + 60 {
        let k = key_text(&at(r, key));
        if k.is_empty() {
            blank_run += 1;
        } else {
            blank_run = 0;
            if !k.starts_with("итог") {
                actual.push((r, k));
            }
        }
        r += 1;
    }
    let header = |c: usize| headers.get(c).cloned().unwrap_or_default();
    let note = |k: &str, c: usize| {
        notes.iter().find(|(nk, nc, _)| fold(nk) == fold(k) && *nc == c).map(|(_, _, why)| format!(" — {why}")).unwrap_or_default()
    };
    let mut problems: Vec<String> = Vec::new();
    for want in rows {
        let Some(Expectation::Text(k)) = want.get(key) else { continue };
        let found: Vec<u32> = actual.iter().filter(|(_, a)| *a == fold(k)).map(|(r, _)| *r).collect();
        match found.as_slice() {
            [] => problems.push(tr(&format!("нет строки {k}"))),
            [row] => {
                for (c, exp) in want.iter().enumerate() {
                    let col = cols.get(c).copied().unwrap_or(Col::Given);
                    if c == key || col == Col::Given {
                        continue;
                    }
                    let got = at(*row, c);
                    let h = header(c);
                    let issue = match (col, exp, &got) {
                        (_, _, Value::Empty) => Some(tr(&format!("{k}, «{h}»: пусто"))),
                        (Col::Text, Expectation::Text(t), Value::Text(v)) if tidy(v) == tidy(t) => None,
                        (Col::Text, Expectation::Text(t), v) => Some(tr(&format!("{k}, «{h}»: стоит {}, а нужно «{t}»", show(v)))),
                        (Col::Number | Col::Date, Expectation::Number(n), Value::Number(v)) if (v - n).abs() < 0.005 => None,
                        (Col::Date, Expectation::Number(n), Value::Number(v)) if swapped(*n).is_some_and(|s| (s - v).abs() < 0.5) => {
                            Some(tr(&format!("{k}, «{h}»: перепутаны день и месяц")))
                        }
                        (Col::Date, _, Value::Text(v)) => Some(tr(&format!("{k}, «{h}»: «{v}» — это текст, а нужна настоящая дата"))),
                        (Col::Number, _, Value::Text(v)) => Some(tr(&format!("{k}, «{h}»: «{v}» — это текст, а нужно число"))),
                        (Col::Date, Expectation::Number(n), v) => {
                            Some(tr(&format!("{k}, «{h}»: стоит {}, а нужно {}", show(v), crate::sheet::date_text(*n))))
                        }
                        (_, Expectation::Number(n), v) => Some(tr(&format!("{k}, «{h}»: стоит {}, а нужно {}", show(v), show(&Value::Number(*n))))),
                        _ => None,
                    };
                    if let Some(issue) = issue {
                        problems.push(format!("{issue}{}", note(k, c)));
                    }
                }
            }
            many => problems.push(tr(&format!("{k} записан {} раза — нужна одна строка", many.len()))),
        }
    }
    let expected_keys: Vec<String> = rows
        .iter()
        .filter_map(|w| match w.get(key) {
            Some(Expectation::Text(k)) => Some(fold(k)),
            _ => None,
        })
        .collect();
    for (r, k) in &actual {
        if !expected_keys.contains(k) {
            problems.push(tr(&format!("строка {r}: «{k}» — такой строки в источниках нет")));
        }
    }
    if problems.is_empty() {
        return (true, tr(&format!("Отчёт сходится: {} строк", rows.len())), None);
    }
    let more = problems.len().saturating_sub(4);
    let mut text = problems.into_iter().take(4).collect::<Vec<_>>().join("; ");
    if more > 0 {
        text.push_str(&tr(&format!("; и ещё {more}")));
    }
    (false, text, None)
}
