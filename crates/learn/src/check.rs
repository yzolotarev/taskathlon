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
    /// The cell must match `reference` (canonical syntax) now and on changed source data.
    Formula {
        cell: String,
        reference: String,
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
        /// Last column belonging to each record; omitted for older single-column checks.
        #[serde(default)]
        last_col: Option<u32>,
        first: u32,
        last: u32,
        expected: usize,
    },
    Keys {
        combo: String,
    },
    Today {
        cell: String,
    },
    /// Label → amount pairs somewhere on the sheet, by any method (pivot, formulas).
    Pairs {
        pairs: Vec<(String, String)>,
    },
    /// A number in a cell; with `either_sign`, the sign convention is the learner's choice.
    Amount {
        cell: String,
        expected: f64,
        either_sign: bool,
    },
    /// Documents with a discrepancy found anywhere on the sheet: a cell naming the document
    /// number ("123", "СФ-00123", "№ 123") and, within eight cells to the right, the learner's own
    /// cell with the difference, either sign.
    Findings {
        items: Vec<(u32, f64)>,
    },
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

fn intact(task: &Task, wb: &Workbook) -> bool {
    task.setup.cells.iter().filter(|(_, _, input)| *input).all(|(cell, v, _)| {
        CellRef::parse(cell).is_some_and(|a| {
            let sh = sheet_of(wb);
            sh.is_some_and(|sh| sh.cell(a).is_none_or(|c| c.formula.is_none()) && equivalent(&sh.value(a), &cell_value(v)))
        })
    })
}

pub fn evaluate(task: &Task, s: &Session, pressed: &[String]) -> Result<Evaluation> {
    let wb = s.doc()?.wb.clone();
    let source_ok = task.reorders || intact(task, &wb);
    let mut outcomes = Vec::new();
    for c in &task.checks {
        let (passed, feedback, solution) = match &c.kind {
            Kind::Formula { cell, reference, live } => formula(task, &wb, cell, reference, *live, source_ok)?,
            Kind::Value { cell, expected } => {
                let actual = sheet::value(s, cell)?;
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
                let ok = all_cells(range, |a| sheet_of(&wb).is_some_and(|sh| wb.styles.get(sh.style_id(a)).font.bold));
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
                let mut after: Vec<String> = rows.iter().map(|r| r.iter().map(show).collect::<Vec<_>>().join("|")).collect();
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
                let ok =
                    all_cells(range, |a| sheet_of(&wb).is_some_and(|sh| wb.styles.get(sh.style_id(a)).num_fmt.as_str().contains(contains.as_str())));
                (
                    ok,
                    if ok { ts("Формат применён").into() } else { tr(&format!("У {range} другой формат чисел")) },
                    Some(ts("Ctrl+1 → Числовой, разделитель групп").into()),
                )
            }
            Kind::Unique { col, last_col, first, last, expected } => {
                let mut seen = Vec::new();
                let mut dupes = false;
                let cols = (*col, last_col.unwrap_or(*col));
                let mut original = original_rows(task, *first, *last, cols);
                original.sort();
                original.dedup();
                let mut remaining = Vec::new();
                for r in *first..=*last {
                    let v = at(&wb, r, *col);
                    let record: Vec<_> = (cols.0..=cols.1).map(|c| at(&wb, r, c)).collect();
                    if record.iter().all(|v| matches!(v, Value::Empty)) {
                        continue;
                    }
                    remaining.push(record.iter().map(show).collect::<Vec<_>>().join("|"));
                    let k = show(&v);
                    if seen.contains(&k) {
                        dupes = true;
                    }
                    seen.push(k);
                }
                remaining.sort();
                let intact = remaining == original;
                let ok = !dupes && seen.len() == *expected && intact;
                (
                    ok,
                    if ok {
                        ts("Дубликаты удалены").into()
                    } else if !intact {
                        ts("Сохрани все уникальные исходные строки целиком: названия и связанные с ними данные").into()
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
            Kind::Today { cell } => {
                let actual = sheet::value(s, cell)?;
                let today = Value::number(gridcraft_engine::calc::now_serial().floor());
                let ok = equivalent(&actual, &today);
                (
                    ok,
                    if ok {
                        ts("Сегодняшняя дата введена").into()
                    } else {
                        tr(&format!("Введи сегодняшнюю дату в {cell}"))
                    },
                    Some("Ctrl+; → Enter".into()),
                )
            }
            Kind::Pairs { pairs } => pairs_check(task, &wb, pairs)?,
            Kind::Amount { cell, expected, either_sign } => {
                let actual = sheet::value(s, cell)?;
                let want = Value::number(*expected);
                let ok = equivalent(&actual, &want) || (*either_sign && equivalent(&actual, &Value::number(-*expected)));
                let fb = if ok {
                    ts("Верно").into()
                } else if matches!(actual, Value::Empty) {
                    tr(&format!("{cell} пока пустая"))
                } else {
                    tr(&format!("В {cell} стоит {}, а нужно {}", show(&actual), show(&want)))
                };
                (ok, fb, Some(show(&want)))
            }
            Kind::Findings { items } => findings_check(task, &wb, items),
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
        .map(|r| {
            (cols.0..=cols.1)
                .map(|c| task.setup.value_of(&sheet::a1(r, c)).map(cell_value).map(|v| show(&v)).unwrap_or_else(|| show(&Value::Empty)))
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect()
}

fn all_cells(range: &str, f: impl Fn(CellRef) -> bool) -> bool {
    let Some(r) = RangeRef::parse(range).or_else(|| CellRef::parse(range).map(|c| RangeRef::new(c, c))) else { return false };
    (r.start.row..=r.end.row).all(|row| (r.start.col..=r.end.col).all(|col| f(CellRef::new(row, col))))
}

fn formula(task: &Task, wb: &Workbook, cell: &str, reference: &str, live: bool, source_ok: bool) -> Result<(bool, String, Option<String>)> {
    let solution = Some(sheet::ru(reference));
    let actual = sheet_of(wb).map(|sh| sh.value(sheet::address(cell).unwrap_or_default())).unwrap_or_default();
    let expected = sheet::probe(&task.setup, &[reference])?.into_iter().next().unwrap_or_default();
    let is_formula = has_formula(wb, cell);
    if !source_ok {
        return Ok((false, ts("Исходные данные изменены — их трогать нельзя. Отмени правку (Ctrl+Z)").into(), solution));
    }
    if matches!(actual, Value::Empty) {
        return Ok((false, tr(&format!("{cell} пока пустая")), solution));
    }
    if let Value::Error(e) = &actual {
        return Ok((false, tr(&format!("Формула вернула {}", sheet::error_ru(e.as_str()))), solution));
    }
    if !equivalent(&actual, &expected) {
        return Ok((false, tr(&format!("В {cell} получилось {}, а должно быть {}", show(&actual), show(&expected))), solution));
    }
    if live && !is_formula {
        return Ok((false, ts("Число верное, но вписано вручную. Нужна формула — она пересчитается на новых данных").into(), solution));
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
                return Ok((false, ts("На других данных формула ошибается. Проверь диапазоны и закрепление $").into(), solution));
            }
        }
    }
    Ok((true, ts("Верно!").into(), solution))
}

/// The document number a cell names: a whole number, or the last group of digits in text.
fn doc_number(v: &Value) -> Option<u32> {
    match v {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 && *n < 1e9 => Some(*n as u32),
        Value::Text(t) => {
            let digits: String = t.chars().rev().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect();
            digits.chars().rev().collect::<String>().parse().ok()
        }
        _ => None,
    }
}

fn findings_check(task: &Task, wb: &Workbook, items: &[(u32, f64)]) -> (bool, String, Option<String>) {
    let inputs: Vec<CellRef> =
        task.setup.cells.iter().filter(|(_, _, i)| *i).map(|(c, ..)| c).chain(task.setup.answers.iter()).filter_map(|c| CellRef::parse(c)).collect();
    let Some(sh) = sheet_of(wb) else { return (false, ts("Нет листа").into(), None) };
    let mut missing = Vec::new();
    for (no, diff) in items {
        let found = sh.cells.iter().any(|(a, c)| {
            // The number may be a source cell (a difference column next to the export);
            // the difference must be the learner's own cell.
            doc_number(&c.value) == Some(*no)
                && (1..=8u32).any(|dc| {
                    let at = CellRef::new(a.row, a.col + dc);
                    let v = sh.value(at);
                    !inputs.contains(&at) && (equivalent(&v, &Value::number(*diff)) || equivalent(&v, &Value::number(-*diff)))
                })
        });
        if !found {
            missing.push(no.to_string());
        }
    }
    let solution = Some(items.iter().map(|(no, d)| format!("{no} — {}", show(&Value::number(*d)))).collect::<Vec<_>>().join("; "));
    if missing.is_empty() {
        (true, ts("Все расхождения найдены").into(), solution)
    } else {
        (false, tr(&format!("Не нашёл расхождений по документам: {}", missing.join(", "))), solution)
    }
}

fn pairs_check(task: &Task, wb: &Workbook, pairs: &[(String, String)]) -> Result<(bool, String, Option<String>)> {
    let refs: Vec<&str> = pairs.iter().map(|(_, f)| f.as_str()).collect();
    let expected = sheet::probe(&task.setup, &refs)?;
    // Labels in the source data or in the case's answer cells don't count as a summary.
    let inputs: Vec<CellRef> =
        task.setup.cells.iter().filter(|(_, _, i)| *i).map(|(c, ..)| c).chain(task.setup.answers.iter()).filter_map(|c| CellRef::parse(c)).collect();
    let Some(sh) = sheet_of(wb) else { return Ok((false, ts("Нет листа").into(), None)) };
    let mut missing = Vec::new();
    for ((label, _), want) in pairs.iter().zip(expected.iter()) {
        let mut found = false;
        for (a, c) in sh.cells.iter() {
            if inputs.contains(&a) || !c.value.as_text().is_some_and(|t| fold(t) == fold(label)) {
                continue;
            }
            for dc in 1..=8u32 {
                if equivalent(&sh.value(CellRef::new(a.row, a.col + dc)), want) {
                    found = true;
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
