//! Offscreen screenshot of the actual learning UI, not a mockup.
//! `cargo run -p gridcraft-learn --example snapshot -- out.png [home|lesson|formula|mentor|wrong|done|case|report|stats|sql|blitz|blitzreport]`
//! Optional `SNAP_WIDTH` and `SNAP_HEIGHT` select the viewport for layout checks.
use gridcraft_learn::{LearningApp, Screen};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args().nth(1).unwrap_or_else(|| "learn-snapshot.png".into());
    let which = std::env::args().nth(2).unwrap_or_else(|| "home".into());
    let mut app = LearningApp::new(None)?;
    if std::env::var_os("SNAP_EN").is_some() {
        app.profile.english = true;
        gridcraft_tr::set_english(true);
        app.sheet.ui.language = gridcraft_ui_egui::i18n::Language::En;
        app.greeting = Some(gridcraft_learn::remind::greeting(&app.profile, gridcraft_learn::now()));
    }
    match which.as_str() {
        "lesson" => app.start_lesson(None)?,
        "formula" | "mentor" => {
            app.start_lesson(Some("logic.andor"))?;
            if which == "mentor" {
                if let Some(cell) = app.active.as_ref().and_then(|a| a.task.answer_cells().first().cloned()) {
                    app.sheet.session.execute("cell.set", serde_json::json!({"cell": cell, "input": "=NO_SUCH_FUNCTION(1)"}))?;
                    app.observe_errors();
                }
            }
        }
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
        "case" => app.start_case(&std::env::var("SNAP_CASE").unwrap_or_else(|_| "case.payroll".into()), false)?,
        "report" => {
            app.start_interview()?;
            app.submit_case(false)?;
        }
        "stats" => {
            use gridcraft_learn_core::profile::{Checkpoint, DAY, Phase};
            let t = gridcraft_learn::now();
            app.profile.checkpoints = vec![
                Checkpoint { phase: Phase::Before, at: t - 12 * DAY, solved: 3, total: 8, seconds: 640.0 },
                Checkpoint { phase: Phase::After, at: t - 8 * DAY, solved: 7, total: 8, seconds: 410.0 },
                Checkpoint { phase: Phase::Later, at: t, solved: 7, total: 8, seconds: 380.0 },
            ];
            app.profile.method_right = 11;
            app.profile.method_total = 14;
            app.screen = Screen::Stats;
        }
        "checkpoint" => app.start_checkpoint()?,
        "transfer" => {
            app.start_transfer()?;
            app.check_transfer()?;
        }
        "checkdone" => {
            app.start_checkpoint()?;
            for _ in 0..8 {
                if app.screen != Screen::Lesson {
                    break;
                }
                app.advance()?;
            }
        }
        "choice" | "choicepicked" => {
            app.start_lesson(Some("cond.sumif"))?;
            if let Some(a) = app.active.as_mut() {
                a.task.spec.level = 3;
                a.task.choices = gridcraft_learn_core::content::recognition(&a.task.spec).into_iter().collect();
            }
            if which == "choicepicked" {
                app.pick_choice(0);
            }
        }
        "sql" => {
            app.start_lesson(Some("sql.group"))?;
            if let Some(a) = app.active.as_mut() {
                a.query = "SELECT category, SUM(amount)\nFROM orders\nGROUP BY category".into();
            }
            app.run_sql();
        }
        "ledger" | "ledgerdoc" | "ledgertb" | "ledgercard" | "ledgerpractice" => {
            let mut v = gridcraft_learn::ledger_ui::LedgerView::load(None);
            match which.as_str() {
                "ledgerdoc" => {
                    let d = v.book.docs.iter().find(|d| d.body.kind() == gridcraft_ledger::DocKind::Sale).cloned();
                    v.draft = d.map(|d| (d, false));
                    v.section = gridcraft_learn::ledger_ui::Section::Sales;
                }
                "ledgerpractice" => {
                    v.start_practice(7);
                    if let Some(pr) = v.practice.as_mut() {
                        pr.show_hint = true;
                    }
                }
                "ledgertb" => v.section = gridcraft_learn::ledger_ui::Section::Reports,
                "ledgercard" => {
                    v.section = gridcraft_learn::ledger_ui::Section::Reports;
                    v.report = gridcraft_learn::ledger_ui::ReportKind::Card;
                    v.account = "62".into();
                }
                _ => {
                    v.selected = v.book.docs.get(2).map(|d| d.id);
                    v.show_entries = true;
                }
            }
            app.ledger = Some(v);
            app.screen = Screen::Ledger;
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
    let width = std::env::var("SNAP_WIDTH").ok().and_then(|s| s.parse::<f32>().ok()).unwrap_or(1280.0);
    let height = std::env::var("SNAP_HEIGHT").ok().and_then(|s| s.parse::<f32>().ok()).unwrap_or(820.0);
    let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(width, height)).with_max_steps(20).wgpu().build_ui_state(
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
