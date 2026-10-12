//! Conservative action diagnosis from the submitted workbook, not just its score.
use crate::{Evaluation, Kind, Profile, Result, Task};
use gridcraft_engine::{Session, core::CellRef, model::Formula};

pub fn prerequisites(id: &str) -> &'static [&'static str] {
    match id {
        "logic.andor" => &["logic.if"],
        "cond.sumifs" => &["cond.sumif", "logic.andor"],
        "cond.sumif" | "cond.countif" => &["logic.if"],
        "refs.abs" => &["refs.fill"],
        _ => &[],
    }
}

fn evidence(profile: &mut Profile, action: &str, ok: bool) {
    let p = profile.actions.entry(action.into()).or_default();
    p.attempts = p.attempts.saturating_add(1);
    p.successes = p.successes.saturating_add(u32::from(ok));
}

fn retry(profile: &mut Profile, skill: &str) {
    if !profile.remediation.iter().any(|s| s == skill) {
        profile.remediation.push(skill.into());
    }
}

/// Called only on an ordinary drill's first submission. Exact comparator reversals
/// are recognised; other wrong formulas retain their normal feedback.
pub fn record(profile: &mut Profile, task: &Task, session: &Session, keys: &[String], eval: &mut Evaluation) -> Result<()> {
    let wb = &session.doc()?.wb;
    let Some(sh) = wb.sheet(wb.active_sheet) else { return Ok(()) };
    // Modified input makes a diagnosis ambiguous; the grader already explains that.
    if !task.reorders
        && task.setup.cells.iter().filter(|(_, _, input)| *input).any(|(cell, expected, _)| {
            CellRef::parse(cell).is_none_or(|at| {
                let actual = sh.value(at);
                match expected {
                    crate::sheet::Cell::Num(n) | crate::sheet::Cell::Date(n) => {
                        !crate::check::equivalent(&actual, &gridcraft_engine::core::Value::number(*n))
                    }
                    crate::sheet::Cell::Text(t) => !crate::check::equivalent(&actual, &gridcraft_engine::core::Value::text(t.as_str())),
                    crate::sheet::Cell::Formula(_) => false,
                }
            })
        })
    {
        return Ok(());
    }
    let formulas: Vec<_> = task
        .checks
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match &c.kind {
            Kind::Formula { cell, reference, .. } => Some((i, cell, reference)),
            _ => None,
        })
        .collect();
    if formulas.is_empty() {
        return Ok(());
    }
    let mut comparison_wrong = false;
    let mut comparison_present = false;
    for (i, cell, reference) in &formulas {
        let canonical = Formula::new(reference).text;
        let actual = CellRef::parse(cell).and_then(|at| sh.cell(at)).and_then(|c| c.formula.as_ref());
        let Some(actual) = actual else { continue };
        if canonical.contains('<') || canonical.contains('>') {
            comparison_present = true;
            if eval.outcomes.get(*i).is_some_and(|o| !o.passed) {
                // Flip one operator at a time; preserve all other formula semantics.
                let reversed = canonical.char_indices().filter(|(_, c)| *c == '<' || *c == '>').any(|(at, c)| {
                    // <=, >= and <> are not a strict reversed comparator.
                    if canonical.as_bytes().get(at + 1).is_some_and(|b| *b == b'=' || *b == b'>') || at > 0 && canonical.as_bytes()[at - 1] == b'<' {
                        return false;
                    }
                    let mut candidate = canonical.clone();
                    candidate.replace_range(at..at + 1, if c == '<' { ">" } else { "<" });
                    candidate == actual.text
                });
                if reversed {
                    comparison_wrong = true;
                    if let Some(o) = eval.outcomes.get_mut(*i) {
                        o.feedback = gridcraft_tr::t(&format!(
                            "{cell}: знак сравнения развёрнут. Проверь, какая величина должна быть меньше, а какая больше."
                        ));
                    }
                }
            }
        }
    }
    if comparison_wrong {
        evidence(profile, "comparison", false);
        retry(profile, "logic.if");
    } else if comparison_present && eval.passed() {
        evidence(profile, "comparison", true);
    }
    if formulas.len() > 1 {
        let (_, first, _) = formulas[0];
        let first_at = CellRef::parse(first);
        let same_column = first_at.is_some_and(|a| formulas.iter().all(|(_, cell, _)| CellRef::parse(cell).is_some_and(|b| a.col == b.col)));
        let all_filled = formulas.iter().all(|(_, cell, _)| CellRef::parse(cell).is_some_and(|at| sh.cell(at).is_some_and(|c| c.formula.is_some())));
        // Confirm relative references actually copied from the first formula.
        let copied = first_at.and_then(|a| sh.cell(a).and_then(|c| c.formula.as_ref()).and_then(|f| f.expr())).is_some_and(|expr| {
            formulas.iter().all(|(_, cell, _)| {
                CellRef::parse(cell).is_some_and(|at| {
                    let a = first_at.unwrap_or(at);
                    let expected = gridcraft_engine::formula::print(&gridcraft_engine::formula::adjust::shift_relative(
                        expr.clone(),
                        i64::from(at.row) - i64::from(a.row),
                        0,
                    ));
                    sh.cell(at).and_then(|c| c.formula.as_ref()).is_some_and(|f| f.text == expected)
                })
            })
        });
        if same_column && all_filled && copied {
            evidence(profile, "refs.fill", true);
            if keys.iter().any(|k| k == "Ctrl+D") {
                evidence(profile, "keys.filldown", true);
            }
        } else if same_column && eval.outcomes.get(formulas[0].0).is_some_and(|o| o.passed) && !all_filled {
            evidence(profile, "refs.fill", false);
            retry(profile, "refs.fill");
        }
    }
    Ok(())
}
