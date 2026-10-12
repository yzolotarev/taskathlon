//! Offscreen screenshot of the actual learning UI, not a mockup.
//! `cargo run -p gridcraft-learn --example snapshot -- out.png [home|lesson|wrong|done|case|report|stats|sql|blitz|blitzreport]`
use gridcraft_learn::{LearningApp, Screen};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args().nth(1).unwrap_or_else(|| "learn-snapshot.png".into());
    let which = std::env::args().nth(2).unwrap_or_else(|| "home".into());
    let mut app = LearningApp::new(None)?;
    match which.as_str() {
        "lesson" => app.start_lesson(None)?,
        "wrong" => {
            app.start_lesson(None)?;
            if let Some(cell) = app.active.as_ref().and_then(|a| a.task.answer_cells().first().cloned()) {
                app.sheet.session.execute("cell.set", serde_json::json!({"cell": cell, "value": 1}))?;
            }
            app.check()?;
        }
        "done" => {
            app.start_lesson(None)?;
            for _ in 0..12 {
                if app.screen != Screen::Lesson {
                    break;
                }
                app.advance()?;
            }
        }
        "case" => app.start_case("case.payroll", false)?,
        "report" => {
            app.start_interview()?;
            app.submit_case(false)?;
        }
        "stats" => app.screen = Screen::Stats,
        "sql" => {
            app.start_lesson(Some("sql.group"))?;
            if let Some(a) = app.active.as_mut() {
                a.query = "SELECT category, SUM(amount)\nFROM orders\nGROUP BY category".into();
            }
            app.run_sql();
        }
        "blitz" => {
            app.start_blitz(gridcraft_learn_core::quiz::Track::Mixed);
            app.blitz_answer(Some(0));
        }
        "blitzreport" => {
            app.start_blitz(gridcraft_learn_core::quiz::Track::Sql);
            for i in 0..20 {
                app.blitz_answer(Some(i % 3));
                app.blitz_next();
            }
        }
        _ => {}
    }
    let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(1280.0, 820.0)).with_max_steps(20).wgpu().build_ui_state(
        |ui, app: &mut LearningApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    harness.run_steps(8);
    harness.render()?.save(output)?;
    Ok(())
}
