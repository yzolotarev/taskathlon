//! Transfer to real Excel: a task goes out as an .xlsx file, the learner solves it in Excel (or
//! any spreadsheet), saves, and the same checks grade the file that comes back.

use crate::{Evaluation, Result, Task, evaluate, sheet};
use gridcraft_engine::Session;
use gridcraft_tr::ts;
use serde_json::json;
use std::path::Path;

/// Writes the task's workbook: the data sheet first, the brief and what to submit on a second sheet.
pub fn export(task: &Task, path: &Path) -> Result<()> {
    let mut s = task.session()?;
    s.execute("sheet.rename", json!({"sheet": 0, "name": ts("Данные")}))?;
    s.execute("home.insertSheet", json!({"name": ts("Задание")}))?;
    s.execute("sheet.activate", json!({"sheet": ts("Задание")}))?;
    let mut lines: Vec<String> = Vec::new();
    if let Some(b) = &task.brief {
        lines.push(format!("{} — {}", b.from, b.subject));
        lines.push(String::new());
        lines.push(b.body.clone());
    } else {
        lines.push(task.title.clone());
    }
    lines.push(String::new());
    lines.push(ts("Что сдать (на листе «Данные», ответы — в жёлтые ячейки):").into());
    lines.extend(task.prompt.lines().map(str::to_string));
    lines.push(String::new());
    lines.push(
        ts("Реши в Excel, сохрани файл как .xlsx и нажми «Проверить файл» в Taskathlon. Имя листа «Данные» и расположение исходных данных не меняй.")
            .into(),
    );
    for (i, l) in lines.iter().enumerate() {
        if !l.is_empty() {
            sheet::set(&mut s, &format!("A{}", i + 1), &sheet::Cell::Text(l.clone()))?;
        }
    }
    s.execute("home.columnWidth", json!({"cols": "A:A", "width": 900.0}))?;
    s.execute("sheet.activate", json!({"sheet": 0}))?;
    s.execute("file.saveAs", json!({"path": path}))?;
    Ok(())
}

/// Opens the file the learner saved and grades it with the task's checks.
pub fn grade(task: &Task, path: &Path) -> Result<Evaluation> {
    let mut s = Session::new();
    s.execute("file.open", json!({"path": path}))?;
    // The checks read the data sheet, whichever sheet was open when the file was saved.
    s.execute("sheet.activate", json!({"sheet": 0}))?;
    evaluate(task, &s, &[])
}
