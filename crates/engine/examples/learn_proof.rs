//! M1 integration proof using the real command and calculation engine.
use gridcraft_engine::{
    DocState, Session,
    core::{CellRef, Value},
};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("range.setValues", json!({"range":"A1", "values":[["Цена","Количество","Выручка"],[120,3,null]]}))?;
    session.execute("cell.set", json!({"cell":"C2", "input":"=B2*A2"}))?;
    let at = CellRef::parse("C2").ok_or("invalid cell address")?;
    let original = session.doc()?.wb.clone();
    let result = original.active().ok_or("missing sheet")?.value(at);
    if result != Value::number(360.0) {
        return Err(format!("wrong initial result: {result:?}").into());
    }
    let mut probe = Session::new();
    probe.add_document(DocState::new((*original).clone(), None, "Probe".into()));
    probe.execute("cell.set", json!({"cell":"A2", "input":"75"}))?;
    if probe.doc()?.wb.active().ok_or("missing probe sheet")?.value(at) != Value::number(225.0) {
        return Err("changed-input calculation failed".into());
    }
    if session.doc()?.wb.active().ok_or("missing original sheet")?.value(at) != Value::number(360.0) {
        return Err("probe modified original".into());
    }
    println!("PASS: create, range write, formula, recalc, read, changed-input recalc, isolation");
    Ok(())
}
