#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::{
    check::*,
    sheet::{self, Cell, Setup},
    *,
};
use gridcraft_engine::{
    core::{CellRef, Value},
    model::{Sheet, Workbook},
};
use serde_json::json;
use std::sync::Arc;

fn task(kind: Kind) -> Task {
    let mut task = Task::generate(&Spec::new("start.enter", 1, 7)).unwrap();
    task.setup = Setup::default();
    task.setup.input("A1", Cell::Num(2.0));
    task.focus = "B1".into();
    task.reorders = false;
    task.checks = vec![Check { label: "test".into(), skill: "start.enter".into(), kind }];
    task
}

#[test]
fn typed_expectations_and_explicit_source_policy() {
    let task = task(Kind::Expected { cell: "B1".into(), expected: Expectation::Number(4.0), formula: false });
    let mut session = task.session().unwrap();
    // Plain input "4" is parsed as a number; this formula produces real text.
    sheet::set(&mut session, "B1", &Cell::Formula("=\"4\"".into())).unwrap();
    assert!(!evaluate(&task, &session, &[]).unwrap().passed());
    sheet::set(&mut session, "B1", &Cell::Num(4.0)).unwrap();
    assert!(evaluate(&task, &session, &[]).unwrap().passed());
    sheet::set(&mut session, "A1", &Cell::Num(3.0)).unwrap();
    assert!(!evaluate(&task, &session, &[]).unwrap().passed());
    assert!(evaluate_with_policy(&task, &session, &[], &SourcePolicy { editable: vec!["A1".into()] }).unwrap().passed());
}

#[test]
fn selection_requires_observed_state_not_keys() {
    let task = task(Kind::Selection { range: "A1:B2".into() });
    let session = task.session().unwrap();
    assert!(!evaluate(&task, &session, &["Ctrl+A".into()]).unwrap().passed());
    assert!(!evaluate_with_observation(&task, &session, &[], &Observation { selection: Some("A1".into()) }).unwrap().passed());
    assert!(evaluate_with_observation(&task, &session, &[], &Observation { selection: Some("$A$1:$B$2".into()) }).unwrap().passed());
}

#[test]
fn qualified_cells_create_sheets_and_preserve_active_tab() {
    let mut setup = Setup::default();
    setup.input("'Other data'!A1", Cell::Num(9.0));
    let mut session = setup.session("A1").unwrap();
    assert_eq!(session.doc().unwrap().wb.active_sheet, 0);
    assert_eq!(sheet::value(&session, "'Other data'!A1").unwrap(), Value::number(9.0));
    sheet::set(&mut session, "'Other data'!B1", &Cell::Formula("=A1*2".into())).unwrap();
    assert_eq!(sheet::value(&session, "'Other data'!B1").unwrap(), Value::number(18.0));
    assert_eq!(session.doc().unwrap().wb.active_sheet, 0);
    assert!(sheet::value(&session, "Missing!A1").is_err());
}

#[test]
fn probe_does_not_overwrite_source_in_old_scratch_column() {
    let mut setup = Setup::default();
    setup.input("ZZ1", Cell::Num(11.0));
    setup.input("XFD1", Cell::Num(7.0));
    assert_eq!(sheet::probe(&setup, &["=ZZ1+XFD1"]).unwrap(), vec![Value::number(18.0)]);
}

fn saved(wb: Workbook, elapsed: f32) -> Vec<u8> {
    let task = task(Kind::Value { cell: "B1".into(), expected: Cell::Num(4.0) });
    serde_json::to_vec(&json!({"version": SAVE_VERSION, "profile": Profile::default(), "current": {
        "task": task, "workbook": wb, "lesson": null, "interview": null, "elapsed": elapsed, "hints": 0
    }}))
    .unwrap()
}

#[test]
fn old_save_defaults_and_multi_sheet_roundtrip() {
    let mut wb = Workbook::new();
    wb.sheets.push(Arc::new(Sheet::new("Summary")));
    wb.active_sheet = 1;
    let save = Save::decode(&saved(wb, 2.0)).unwrap();
    let current = save.current.as_ref().unwrap();
    assert!(current.keys.is_empty());
    assert!(current.query.is_empty());
    assert_eq!(Save::decode(&save.encode().unwrap()).unwrap().current.unwrap().workbook.sheets.len(), 2);
    assert!(Save::decode(&saved(Workbook::new(), -1.0)).is_err());
}

#[test]
fn save_bounds_cover_every_sheet_and_total() {
    let mut wb = Workbook::new();
    wb.sheets.push(Arc::new(Sheet::new("Other")));
    for i in 0..2 {
        for row in 0..25_001 {
            wb.sheet_mut(i).unwrap().set_value(CellRef::new(row, 0), Value::number(1.0));
        }
    }
    assert!(!valid_exercise_workbook(&wb));
    let mut wb = Workbook::new();
    wb.active_sheet = 2;
    assert!(!valid_exercise_workbook(&wb));
    wb.active_sheet = 0;
    wb.sheet_mut(0).unwrap().set_value(CellRef::new(u32::MAX, 0), Value::number(1.0));
    assert!(!valid_exercise_workbook(&wb));
    wb.sheets.clear();
    assert!(!valid_exercise_workbook(&wb));
}

#[test]
fn malformed_checks_do_not_underflow_or_vacuously_pass() {
    let task = task(Kind::FilterShows { col: 0, first: 0, last: 2, value: "x".into() });
    assert!(evaluate(&task, &task.session().unwrap(), &[]).is_err());
    let task = self::task(Kind::Sorted { first: 1, last: 2, cols: (0, 1), key: 2, descending: false });
    assert!(evaluate(&task, &task.session().unwrap(), &[]).is_err());
}

#[test]
fn pivot_check_requires_actual_matching_pivot_definition() {
    use gridcraft_engine::model::PivotFunc;
    let mut task = task(Kind::Pivot {
        sheet: "Sheet1".into(),
        source: "Sheet1!A1:B3".into(),
        rows: vec!["Region".into()],
        columns: vec![],
        values: vec![("Amount".into(), PivotFunc::Sum)],
    });
    task.setup = Setup::default();
    task.setup.text("A1", "Region");
    task.setup.text("B1", "Amount");
    task.setup.input("A2", Cell::Text("East".into()));
    task.setup.input("B2", Cell::Num(10.0));
    task.setup.input("A3", Cell::Text("West".into()));
    task.setup.input("B3", Cell::Num(20.0));
    let mut session = task.session().unwrap();
    sheet::set(&mut session, "Y1", &Cell::Text("East".into())).unwrap();
    sheet::set(&mut session, "Z1", &Cell::Num(10.0)).unwrap();
    assert!(!evaluate(&task, &session, &[]).unwrap().passed());
    session
        .execute(
            "insert.pivotTable",
            json!({"source": "Sheet1!A1:B3", "destination": "Sheet1!H1", "rows": ["Region"], "values": [{"field": "Amount", "func": "sum"}]}),
        )
        .unwrap();
    assert!(evaluate(&task, &session, &[]).unwrap().passed());
    let wb = Arc::make_mut(&mut session.doc_mut().unwrap().wb);
    wb.sheet_mut(0).unwrap().pivots[0].values[0].func = PivotFunc::Count;
    assert!(!evaluate(&task, &session, &[]).unwrap().passed());
}
