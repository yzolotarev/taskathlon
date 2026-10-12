//! SQL drills: small generated databases, run in an in-memory SQLite (native builds only).
//! Grading compares the learner's result set with the reference query's, on the shown
//! data and on other data with the same layout, so a hard-coded answer does not pass.

use crate::check::{Evaluation, Outcome};
use crate::{LearnError, Result, Task};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SqlVal {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
}

impl SqlVal {
    pub fn show(&self) -> String {
        match self {
            Self::Null => "NULL".into(),
            Self::Int(i) => i.to_string(),
            Self::Real(r) => {
                let s = format!("{:.2}", r);
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            }
            Self::Text(t) => t.clone(),
        }
    }
    fn key(&self) -> String {
        match self {
            Self::Null => "\u{0}null".into(),
            Self::Int(i) => format!("n{:.2}", *i as f64),
            Self::Real(r) => format!("n{:.2}", (r * 100.0).round() / 100.0 + 0.0),
            Self::Text(t) => format!("t{}", t.trim()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SqlTable {
    pub name: String,
    /// (column, SQLite type: INTEGER, REAL or TEXT).
    pub columns: Vec<(String, String)>,
    pub rows: Vec<Vec<SqlVal>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SqlTask {
    pub tables: Vec<SqlTable>,
    pub reference: String,
    /// The question asks for an order (ORDER BY … LIMIT), so row order is graded.
    pub ordered: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ResultSet {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<SqlVal>>,
    /// More rows existed than were kept.
    pub truncated: bool,
}

const MAX_ROWS: usize = 2000;

/// Runs one read query against a fresh in-memory database built from `tables`.
#[cfg(not(target_arch = "wasm32"))]
#[expect(clippy::disallowed_methods, reason = "native-only: the runaway-query timer needs a clock")]
pub fn run(tables: &[SqlTable], query: &str) -> std::result::Result<ResultSet, String> {
    use rusqlite::{Connection, types::ValueRef};
    let q = query.trim().trim_end_matches(';').trim();
    if q.is_empty() {
        return Err("Запрос пустой".into());
    }
    let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
    for t in tables {
        let cols = t.columns.iter().map(|(c, ty)| format!("\"{c}\" {ty}")).collect::<Vec<_>>().join(", ");
        conn.execute(&format!("CREATE TABLE \"{}\" ({cols})", t.name), []).map_err(|e| e.to_string())?;
        let marks = vec!["?"; t.columns.len()].join(", ");
        let mut ins = conn.prepare(&format!("INSERT INTO \"{}\" VALUES ({marks})", t.name)).map_err(|e| e.to_string())?;
        for row in &t.rows {
            let params: Vec<rusqlite::types::Value> = row
                .iter()
                .map(|v| match v {
                    SqlVal::Null => rusqlite::types::Value::Null,
                    SqlVal::Int(i) => rusqlite::types::Value::Integer(*i),
                    SqlVal::Real(r) => rusqlite::types::Value::Real(*r),
                    SqlVal::Text(s) => rusqlite::types::Value::Text(s.clone()),
                })
                .collect();
            ins.execute(rusqlite::params_from_iter(params)).map_err(|e| e.to_string())?;
        }
    }
    // A runaway query (recursive CTE, huge cross join) is stopped after two seconds.
    let started = std::time::Instant::now();
    conn.progress_handler(20_000, Some(move || started.elapsed() > std::time::Duration::from_secs(2)));
    let mut stmt = conn.prepare(q).map_err(|e| translate(&e.to_string()))?;
    if !stmt.readonly() {
        return Err("Здесь только чтение: запрос должен начинаться с SELECT или WITH".into());
    }
    let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let n = columns.len();
    let mut rows = stmt.query([]).map_err(|e| translate(&e.to_string()))?;
    let mut out = ResultSet { columns, rows: Vec::new(), truncated: false };
    loop {
        match rows.next() {
            Ok(Some(row)) => {
                if out.rows.len() >= MAX_ROWS {
                    out.truncated = true;
                    break;
                }
                let mut vals = Vec::with_capacity(n);
                for i in 0..n {
                    let v = match row.get_ref(i) {
                        Ok(ValueRef::Null) | Err(_) => SqlVal::Null,
                        Ok(ValueRef::Integer(x)) => SqlVal::Int(x),
                        Ok(ValueRef::Real(x)) => SqlVal::Real(x),
                        Ok(ValueRef::Text(t)) => SqlVal::Text(String::from_utf8_lossy(t).into_owned()),
                        Ok(ValueRef::Blob(_)) => SqlVal::Text("<blob>".into()),
                    };
                    vals.push(v);
                }
                out.rows.push(vals);
            }
            Ok(None) => break,
            Err(e) => return Err(translate(&e.to_string())),
        }
    }
    Ok(out)
}

#[cfg(target_arch = "wasm32")]
pub fn run(_tables: &[SqlTable], _query: &str) -> std::result::Result<ResultSet, String> {
    Err("SQL-тренажёр доступен в настольной версии".into())
}

/// SQLite error text → a short Russian hint (the original stays for search).
fn translate(e: &str) -> String {
    let hint = if e.contains("no such column") {
        "Нет такого столбца — проверь имя (и псевдоним таблицы)."
    } else if e.contains("no such table") {
        "Нет такой таблицы — имена таблиц видны слева."
    } else if e.contains("syntax error") {
        "Синтаксическая ошибка — проверь запятые, кавычки и порядок SELECT … FROM … WHERE … GROUP BY … HAVING … ORDER BY."
    } else if e.contains("ambiguous column") {
        "Столбец есть в двух таблицах — укажи таблицу: o.id или c.id."
    } else if e.contains("misuse of aggregate") {
        "Агрегат (SUM, COUNT…) нельзя ставить в WHERE — для условий по группам есть HAVING."
    } else if e.contains("interrupted") {
        "Запрос выполнялся слишком долго и остановлен."
    } else if e.contains("Multiple statements") {
        "Нужен один запрос (без второго после «;»)."
    } else if e.contains("must appear in the GROUP BY") {
        "Столбец не в GROUP BY и не в агрегате."
    } else {
        "Ошибка в запросе."
    };
    format!("{hint}\n({e})")
}

fn normalized(rs: &ResultSet, ordered: bool) -> Vec<String> {
    let mut rows: Vec<String> = rs.rows.iter().map(|r| r.iter().map(SqlVal::key).collect::<Vec<_>>().join("\u{1}")).collect();
    if !ordered {
        rows.sort();
    }
    rows
}

/// Does `got` answer the same question as `want`? Column names are free; column order counts.
pub fn same(got: &ResultSet, want: &ResultSet, ordered: bool) -> std::result::Result<(), String> {
    if got.columns.len() != want.columns.len() {
        return Err(format!("Столбцов {} , а нужно {}: {}", got.columns.len(), want.columns.len(), want.columns.join(", ")));
    }
    if got.rows.len() != want.rows.len() {
        return Err(format!("Строк {}, а нужно {}", got.rows.len(), want.rows.len()));
    }
    let (a, b) = (normalized(got, ordered), normalized(want, ordered));
    if a == b {
        return Ok(());
    }
    if ordered && normalized(got, false) == normalized(want, false) {
        return Err("Строки те, но порядок другой — проверь ORDER BY (ASC/DESC)".into());
    }
    Err("Значения не совпадают с ожидаемыми".into())
}

/// Grades a query: same result as the reference now and on two other data sets.
pub fn evaluate(task: &Task, query: &str) -> Result<Evaluation> {
    let sql = task.sql.as_ref().ok_or_else(|| LearnError::Invalid("Это не SQL-задание".into()))?;
    let skill = task.spec.skill.clone();
    let outcome = |passed: bool, feedback: String| Outcome {
        label: "Запрос".into(),
        skill: skill.clone(),
        passed,
        feedback,
        solution: Some(sql.reference.clone()),
    };
    let want = run(&sql.tables, &sql.reference).map_err(LearnError::Invalid)?;
    let got = match run(&sql.tables, query) {
        Ok(r) => r,
        Err(e) => return Ok(Evaluation { score: 0.0, outcomes: vec![outcome(false, e)] }),
    };
    if let Err(e) = same(&got, &want, sql.ordered) {
        return Ok(Evaluation { score: 0.0, outcomes: vec![outcome(false, e)] });
    }
    for p in 1..=2u64 {
        let mut spec = task.spec.clone();
        spec.data = spec.data.wrapping_add(p.wrapping_mul(7919));
        let Ok(other) = crate::content::build(&spec) else { continue };
        let Some(osql) = other.sql else { continue };
        let Ok(want) = run(&osql.tables, &osql.reference) else { continue };
        let ok = run(&osql.tables, query).ok().is_some_and(|got| same(&got, &want, osql.ordered).is_ok());
        if !ok {
            return Ok(Evaluation {
                score: 0.0,
                outcomes: vec![outcome(false, "На этих данных совпало, а на других — нет. Не подставляй готовые числа и id: пиши условие".into())],
            });
        }
    }
    Ok(Evaluation { score: 1.0, outcomes: vec![outcome(true, "Результат совпал!".into())] })
}
