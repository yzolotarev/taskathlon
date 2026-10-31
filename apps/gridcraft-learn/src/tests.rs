//! The learning flow through the real UI: one key to start, auto-check, Russian formulas.

use super::*;
use egui_kittest::Harness;
use gridcraft_learn_core::{
    Spec,
    check::Kind,
    sheet::{Cell, ru},
};
use serde_json::json;

fn harness(app: LearningApp) -> Harness<'static, LearningApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1280.0, 820.0)).with_max_steps(20).build_ui_state(
        |ui, app: &mut LearningApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    h.run_steps(6);
    h
}

fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers }
}

/// The host calls `raw_input_hook` before each frame; the test harness does not.
fn press(h: &mut Harness<'static, LearningApp>, k: egui::Key, m: egui::Modifiers) {
    let mut raw = egui::RawInput { events: vec![key(k, m)], ..Default::default() };
    h.state_mut().raw_input_hook(&mut raw);
    h.input_mut().events.extend(raw.events);
    h.run_steps(4);
}

/// Solves the current task with its reference answers, through engine commands.
fn solve(app: &mut LearningApp) {
    let Some(a) = app.active.as_ref() else { return };
    let checks = a.task.checks.clone();
    for c in checks {
        let params = match c.kind {
            Kind::Formula { cell, reference, .. } => json!({"cell": cell, "input": reference}),
            Kind::Value { cell, expected: Cell::Text(t) } => json!({"cell": cell, "value": t}),
            Kind::Value { cell, expected: Cell::Num(n) | Cell::Date(n) } => json!({"cell": cell, "value": n}),
            Kind::Value { cell, expected: Cell::Formula(f) } => json!({"cell": cell, "input": f}),
            _ => continue,
        };
        let _ = app.sheet.session.execute("cell.set", params);
    }
}

fn supply_test_dir(name: &str) -> PathBuf {
    let scratch = Path::new("/home/y/.hermes/cache/scratch/opencode");
    let root = if scratch.is_dir() { scratch.to_path_buf() } else { std::env::temp_dir() };
    root.join(format!("{name}-{}", std::process::id()))
}

fn seed_of(app: &LearningApp) -> u64 {
    app.active.as_ref().expect("an active task").task.spec.layout
}

fn num_at(app: &LearningApp, cell: &str) -> f64 {
    match gridcraft_learn_core::sheet::value(&app.sheet.session, cell).expect("a readable cell") {
        gridcraft_engine::core::Value::Number(n) => n,
        other => panic!("{cell} is {other:?}, expected a number"),
    }
}

fn text_at(app: &LearningApp, cell: &str) -> String {
    gridcraft_learn_core::sheet::value(&app.sheet.session, cell).expect("a readable cell").as_text().unwrap_or_default().to_string()
}

/// Cells and reference formulas declared by a task.
fn formulas(task: &Task) -> Vec<(String, String)> {
    task.checks
        .iter()
        .filter_map(|c| match &c.kind {
            Kind::Formula { cell, reference, .. } => Some((cell.clone(), reference.clone())),
            _ => None,
        })
        .collect()
}

/// The grand-total cell: the last formula check whose reference uses SUM.
fn sum_cell(task: &Task) -> Option<String> {
    task.checks.iter().rev().find_map(|c| match &c.kind {
        Kind::Formula { cell, reference, .. } if reference.to_ascii_uppercase().contains("SUM") => Some(cell.clone()),
        _ => None,
    })
}

/// A per-unit cost formula that multiplies two cells (not a SUM).
fn first_product_formula(task: &Task) -> (String, String) {
    formulas(task).into_iter().find(|(_, r)| r.contains('*')).expect("a product formula")
}

/// The first composite cost formula, found through the grand-total range.
fn composite_first_cost(task: &Task) -> (String, String) {
    let total =
        formulas(task).into_iter().rev().find(|(_, r)| r.to_ascii_uppercase().contains("SUM")).map(|(_, r)| r).expect("a grand-total SUM formula");
    let body = total.trim_start_matches("=SUM(").trim_end_matches(')');
    let first = body.split(':').next().expect("a range").to_string();
    formulas(task).into_iter().find(|(c, _)| *c == first).expect("the first product formula of the total")
}

/// `=B2*C2` → `=$B$2*$C$2` (column letters get an absolute marker).
fn absolutize(f: &str) -> String {
    let mut out = String::new();
    for ch in f.chars() {
        if ch.is_ascii_uppercase() {
            out.push('$');
        }
        out.push(ch);
    }
    out
}

/// Column letters of `=X*Y`, left (price) and right (quantity) as the route writes them.
fn product_cols(reference: &str) -> (String, String) {
    let body = reference.trim_start_matches('=');
    let (left, right) = body.split_once('*').expect("a product reference");
    let letters = |s: &str| s.chars().filter(|c| c.is_ascii_alphabetic()).collect::<String>();
    (letters(left), letters(right))
}

#[test]
fn supply_workbook_flows_without_banners_and_requires_final_submission() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_supply().unwrap();
    let seed = seed_of(&app);
    let name_cell = app.active.as_ref().unwrap().task.focus.clone();
    let expected_name = match &app.active.as_ref().unwrap().task.checks[0].kind {
        Kind::Value { expected: Cell::Text(t), .. } => t.clone(),
        other => panic!("expected a text check, got {other:?}"),
    };
    assert!(gridcraft_learn_core::sheet::value(&app.sheet.session, &name_cell).unwrap().is_empty());
    for stage in 0..11 {
        assert_eq!(app.supply_stage(), Some(stage));
        solve(&mut app);
        app.check_supply(false).unwrap();
        assert_eq!(app.supply_stage(), Some(stage + 1));
        assert!(app.feedback.is_none());
        assert_eq!(app.profile.xp, 0);
        assert!(!app.sheet.session.doc().unwrap().undo.is_empty());
        assert_eq!(text_at(&app, &name_cell), expected_name);
    }
    app.check_supply(false).unwrap();
    assert!(!app.supply.as_ref().unwrap().complete);
    app.check_supply(true).unwrap();
    assert!(app.supply.as_ref().unwrap().complete);
    // After the manager's change request the grand total recalculated to 254 000.
    let total = sum_cell(&supply::task(11, seed)).unwrap();
    assert_eq!(num_at(&app, &total), 254_000.);
}

#[test]
fn supply_rejects_constant_and_wrong_relative_formulas_on_altered_orders() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_supply().unwrap();
    for _ in 0..6 {
        solve(&mut app);
        app.check_supply(false).unwrap();
    }
    solve(&mut app);
    let task = app.active.as_ref().unwrap().task.clone();
    assert!(supply::valid(&task, &app.sheet.session).unwrap());
    let list = formulas(&task);
    let (cell, reference) = list.last().cloned().expect("a fill-down formula");
    let (first_cell, first_reference) = list.first().cloned().expect("the first formula");
    assert_ne!(cell, first_cell);

    // A formula that does not follow the data fails the altered-data probe.
    app.sheet.session.execute("cell.set", json!({"cell": cell, "input": "=400"})).unwrap();
    assert!(!supply::valid(&task, &app.sheet.session).unwrap());

    // Absolute references on the correct row are still a working row formula.
    app.sheet.session.execute("cell.set", json!({"cell": cell, "input": absolutize(&reference)})).unwrap();
    assert!(supply::valid(&task, &app.sheet.session).unwrap());

    // A relative reference to the wrong row fails.
    app.sheet.session.execute("cell.set", json!({"cell": cell, "input": first_reference})).unwrap();
    assert!(!supply::valid(&task, &app.sheet.session).unwrap());
}

#[test]
fn supply_real_filldown_creates_relative_formulas_and_updates_total() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_supply().unwrap();
    let seed = seed_of(&app);
    for _ in 0..6 {
        solve(&mut app);
        app.check_supply(false).unwrap();
    }
    assert_eq!(app.supply_stage(), Some(6));
    let fill_range = app.active.as_ref().unwrap().task.focus.clone();
    app.sheet.session.execute("edit.fillDown", json!({"range": fill_range})).unwrap();
    app.check_supply(false).unwrap();
    assert_eq!(app.supply_stage(), Some(7));
    solve(&mut app);
    app.check_supply(false).unwrap();
    assert_eq!(app.supply_stage(), Some(8));
    // The stage-8 quantity change: pick the last numeric value check (the newly requested one).
    let (change_cell, expected) = app
        .active
        .as_ref()
        .unwrap()
        .task
        .checks
        .iter()
        .rev()
        .find_map(|c| match &c.kind {
            Kind::Value { cell, expected: Cell::Num(n) } => Some((cell.clone(), *n)),
            _ => None,
        })
        .expect("the stage-8 change request");
    app.sheet.session.execute("cell.set", json!({"cell": change_cell, "value": expected})).unwrap();
    app.check_supply(false).unwrap();
    assert_eq!(app.supply_stage(), Some(9));
    // The total still equals the sum of the recalculated line costs.
    let t7 = supply::task(7, seed);
    let total_cell = sum_cell(&t7).unwrap();
    let line_sum: f64 = formulas(&t7).into_iter().filter(|(c, _)| *c != total_cell).map(|(c, _)| num_at(&app, &c)).sum();
    assert_eq!(num_at(&app, &total_cell), line_sum);
}

#[test]
fn supply_enter_is_not_consumed_and_pause_blocks_progress() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_supply().unwrap();
    let mut raw = egui::RawInput { events: vec![key(egui::Key::Enter, egui::Modifiers::NONE)], ..Default::default() };
    app.raw_input_hook(&mut raw);
    assert!(!app.enter);
    app.pause_supply();
    let frozen = app.supply.as_ref().unwrap().seconds();
    solve(&mut app);
    app.check_supply(false).unwrap();
    assert_eq!(app.supply_stage(), Some(0));
    assert_eq!(app.supply.as_ref().unwrap().seconds(), frozen);
    app.pause_supply();
    app.check_supply(false).unwrap();
    assert_eq!(app.supply_stage(), Some(1));
}

#[test]
fn supply_first_typing_and_enter_commit_only_one_step_in_the_real_grid() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_supply().unwrap();
    let name_cell = app.active.as_ref().unwrap().task.focus.clone();
    let expected_name = match &app.active.as_ref().unwrap().task.checks[0].kind {
        Kind::Value { expected: Cell::Text(t), .. } => t.clone(),
        other => panic!("expected a text check, got {other:?}"),
    };
    let mut h = harness(app);
    h.input_mut().events.push(egui::Event::Text(expected_name.clone()));
    h.run_steps(3);
    assert!(h.state().sheet.editor.is_some());
    press(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(h.state().supply_stage(), Some(1));
    assert!(h.state().feedback.is_none());
    assert_eq!(text_at(h.state(), &name_cell), expected_name);
    press(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(h.state().supply_stage(), Some(1));
}

#[test]
fn supply_restart_restores_stage_workbook_and_quiet_pause() {
    let dir = supply_test_dir("supply-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("progress.json");
    let mut app = LearningApp::new(Some(path.clone())).unwrap();
    app.start_supply().unwrap();
    let seed = seed_of(&app);
    for _ in 0..5 {
        solve(&mut app);
        app.check_supply(false).unwrap();
    }
    app.supply.as_mut().unwrap().quiet = true;
    app.pause_supply();
    app.save().unwrap();
    let restored = LearningApp::new(Some(path.clone())).unwrap();
    assert_eq!(restored.screen, Screen::Supply);
    assert_eq!(restored.supply_stage(), Some(5));
    assert!(restored.supply.as_ref().unwrap().paused);
    assert!(restored.supply.as_ref().unwrap().quiet);
    // The learner's own line cost is price × quantity, in the seeded layout.
    let t5 = supply::task(5, seed);
    let cost = first_product_formula(&t5).0;
    let product: f64 = t5
        .checks
        .iter()
        .filter(|c| matches!(c.kind, Kind::Value { expected: Cell::Num(_), .. }))
        .take(2)
        .filter_map(|c| match &c.kind {
            Kind::Value { expected: Cell::Num(n), .. } => Some(*n),
            _ => None,
        })
        .product();
    assert_eq!(num_at(&restored, &cost), product);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn supply_every_step_and_explicit_completion_survive_restart() {
    let dir = supply_test_dir("supply-all-stages-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("progress.json");
    let mut app = LearningApp::new(Some(path.clone())).unwrap();
    app.start_supply().unwrap();
    let seed = seed_of(&app);
    for stage in 0..12 {
        app.save().unwrap();
        app = LearningApp::new(Some(path.clone())).unwrap();
        assert_eq!(app.screen, Screen::Supply);
        assert_eq!(app.supply_stage(), Some(stage));
        assert_eq!(app.active.as_ref().unwrap().task.prompt, supply::task(stage, seed).prompt);
        if stage > 4 {
            let t5 = supply::task(5, seed);
            let cost = first_product_formula(&t5).0;
            let product: f64 = t5
                .checks
                .iter()
                .filter(|c| matches!(c.kind, Kind::Value { expected: Cell::Num(_), .. }))
                .take(2)
                .filter_map(|c| match &c.kind {
                    Kind::Value { expected: Cell::Num(n), .. } => Some(*n),
                    _ => None,
                })
                .product();
            assert_eq!(num_at(&app, &cost), product);
        }
        if stage < 11 {
            solve(&mut app);
            app.check_supply(false).unwrap();
        }
    }
    app.check_supply(true).unwrap();
    app = LearningApp::new(Some(path.clone())).unwrap();
    assert!(app.supply.as_ref().unwrap().complete);
    assert_eq!(app.supply_stage(), Some(11));
    let cable = supply::task(11, seed)
        .checks
        .iter()
        .find_map(|c| match &c.kind {
            Kind::Value { cell, expected: Cell::Num(45.) } => Some(cell.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(num_at(&app, &cable), 45.);
    let total = sum_cell(&supply::task(11, seed)).unwrap();
    assert_eq!(num_at(&app, &total), 254_000.);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn supply_layout_varies_by_seed_and_is_deterministic() {
    let mut focuses = std::collections::HashSet::new();
    let mut prices = std::collections::HashSet::new();
    for seed in 0..48u64 {
        let a = supply::task(6, seed);
        let b = supply::task(6, seed);
        // Same (stage, seed) reproduces the route exactly.
        assert_eq!(a.spec, b.spec);
        assert_eq!(a.focus, b.focus);
        assert_eq!(a.checks, b.checks);
        assert_eq!(a.setup.cells, b.setup.cells);
        focuses.insert(a.focus.clone());
        let (_, reference) = first_product_formula(&a);
        prices.insert(reference);
    }
    assert!(focuses.len() > 1, "the table start row/column must vary by seed");
    assert!(prices.len() > 1, "prices, quantities and formulas must vary by seed");

    // The price/quantity columns swap for some seeds and not for others.
    let swapped = prices.iter().any(|r| {
        let (left, right) = product_cols(r);
        left > right
    });
    let plain = prices.iter().any(|r| {
        let (left, right) = product_cols(r);
        left < right
    });
    assert!(swapped && plain, "price/quantity column order must vary: {prices:?}");
}

#[test]
fn supply_wrong_step_inserts_variation_then_returns_to_main() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_supply().unwrap();
    let main0 = app.active.as_ref().unwrap().task.clone();
    let name_cell = main0.focus.clone();
    let expected_name = match &main0.checks[0].kind {
        Kind::Value { expected: Cell::Text(t), .. } => t.clone(),
        other => panic!("expected a text check, got {other:?}"),
    };
    app.sheet.session.execute("cell.set", json!({"cell": name_cell, "value": "Неверное"})).unwrap();
    app.check_supply(true).unwrap();

    // The route does not advance: it names the failing cell and drops into a variation.
    assert_eq!(app.supply_stage(), Some(0));
    let fix = app.active.as_ref().unwrap().task.clone();
    assert!(supply::is_fix(&fix));
    assert_ne!(fix.focus, name_cell, "the variation uses a new layout");
    let feedback = app.supply.as_ref().unwrap().feedback.clone().unwrap_or_default();
    assert!(feedback.contains(&name_cell), "must name the failing cell: {feedback}");

    // Solving the variation returns to the main step without advancing.
    solve(&mut app);
    app.check_supply(false).unwrap();
    assert!(!supply::is_fix(&app.active.as_ref().unwrap().task));
    assert_eq!(app.supply_stage(), Some(0));
    assert_eq!(text_at(&app, &name_cell), "Неверное");

    // A corrected main step advances.
    app.sheet.session.execute("cell.set", json!({"cell": name_cell, "value": expected_name})).unwrap();
    app.check_supply(false).unwrap();
    assert_eq!(app.supply_stage(), Some(1));
}

#[test]
fn supply_composite_brief_totals_and_change_request() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_supply().unwrap();
    let seed = seed_of(&app);
    for stage in 0..9 {
        assert_eq!(app.supply_stage(), Some(stage));
        solve(&mut app);
        app.check_supply(false).unwrap();
    }
    assert_eq!(app.supply_stage(), Some(9));
    solve(&mut app);
    app.check_supply(false).unwrap();
    assert_eq!(app.supply_stage(), Some(10));
    let t9 = supply::task(9, seed);
    let total = sum_cell(&t9).unwrap();
    assert_eq!(num_at(&app, &total), 236_000.);

    // The change request: cable 30 → 45, total recalculated automatically.
    solve(&mut app);
    app.check_supply(false).unwrap();
    assert_eq!(app.supply_stage(), Some(11));
    assert_eq!(num_at(&app, &total), 254_000.);
    app.check_supply(true).unwrap();
    assert!(app.supply.as_ref().unwrap().complete);
}

#[test]
fn composite_example_names_are_present_in_generated_task() {
    for seed in [1u64, 2, 3] {
        let task = supply::task(9, seed);
        let texts: Vec<String> = task.setup.cells.iter().filter_map(|(_, v, _)| if let Cell::Text(s) = v { Some(s.clone()) } else { None }).collect();
        for name in ["Датчик", "Кабель", "Контроллер", "Крепёж", "Блок питания"] {
            assert!(texts.iter().any(|s| s == name), "seed {seed}: нет «{name}» среди {texts:?}");
        }
    }
}

#[test]
fn supply_hardcoded_result_is_rejected_on_altered_data() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_supply().unwrap();
    let seed = seed_of(&app);
    for _ in 0..9 {
        solve(&mut app);
        app.check_supply(false).unwrap();
    }
    solve(&mut app);
    let task = app.active.as_ref().unwrap().task.clone();
    assert!(supply::valid(&task, &app.sheet.session).unwrap());
    let (cell, reference) = composite_first_cost(&supply::task(9, seed));

    // A typed constant is not a formula.
    app.sheet.session.execute("cell.set", json!({"cell": cell, "value": 51000.0})).unwrap();
    assert!(!supply::valid(&task, &app.sheet.session).unwrap());

    // A formula pinned to the old total fails the altered-data probe.
    app.sheet.session.execute("cell.set", json!({"cell": cell, "input": "=51000"})).unwrap();
    assert!(!supply::valid(&task, &app.sheet.session).unwrap());

    // The live formula passes and follows the data.
    app.sheet.session.execute("cell.set", json!({"cell": cell, "input": reference})).unwrap();
    assert!(supply::valid(&task, &app.sheet.session).unwrap());
}

#[test]
fn supply_restart_during_correction_restores_the_variation() {
    let dir = supply_test_dir("supply-fix-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("progress.json");
    let mut app = LearningApp::new(Some(path.clone())).unwrap();
    app.start_supply().unwrap();
    let name_cell = app.active.as_ref().unwrap().task.focus.clone();
    app.sheet.session.execute("cell.set", json!({"cell": name_cell, "value": "Мимо"})).unwrap();
    app.check_supply(true).unwrap();
    assert!(supply::is_fix(&app.active.as_ref().unwrap().task));
    app.save().unwrap();

    let mut restored = LearningApp::new(Some(path.clone())).unwrap();
    assert_eq!(restored.screen, Screen::Supply);
    assert_eq!(restored.supply_stage(), Some(0));
    assert!(supply::is_fix(&restored.active.as_ref().unwrap().task));

    solve(&mut restored);
    restored.check_supply(false).unwrap();
    assert!(!supply::is_fix(&restored.active.as_ref().unwrap().task));
    assert_eq!(restored.supply_stage(), Some(0));
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn interview_does_not_start_an_unready_case() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_interview().unwrap();
    assert_eq!(app.screen, Screen::Home);
    assert!(app.active.is_none());
    assert!(app.notice.is_some());
}

#[test]
fn enter_on_home_starts_a_lesson_then_solving_auto_checks_and_enter_advances() {
    let mut h = harness(LearningApp::new(None).unwrap());
    assert_eq!(h.state().screen, Screen::Home);
    press(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(h.state().screen, Screen::Lesson, "{:?}", h.state().notice);
    let first = h.state().active.as_ref().unwrap().task.spec.clone();
    solve(h.state_mut());
    h.run_steps(4);
    let fb = h.state().feedback.as_ref().expect("auto-check after the answer cells are filled");
    assert!(fb.passed, "{:?}", fb.eval.outcomes);
    assert!(fb.xp > 0);
    press(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(h.state().feedback.is_none());
    let next = h.state().active.as_ref().unwrap().task.spec.clone();
    assert_ne!(first, next);
    assert_eq!(h.state().active.as_ref().unwrap().lesson.as_ref().unwrap().done, 1);
}

#[test]
fn a_whole_lesson_reaches_the_done_screen() {
    let mut h = harness(LearningApp::new(None).unwrap());
    press(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
    for _ in 0..30 {
        if h.state().screen != Screen::Lesson {
            break;
        }
        solve(h.state_mut());
        h.run_steps(3);
        if h.state().feedback.is_none() {
            // Action drills (hotkeys, sort…) are skipped here: they are covered in the core tests.
            let r = h.state_mut().advance();
            assert!(r.is_ok());
        } else {
            press(&mut h, egui::Key::Enter, egui::Modifiers::NONE);
        }
    }
    assert_eq!(h.state().screen, Screen::Done);
    assert!(h.state().summary.as_ref().unwrap().xp > 0);
    assert!(h.state().profile.xp > 0);
}

#[test]
fn russian_formula_typed_into_the_grid_passes() {
    let mut app = LearningApp::new(None).unwrap();
    let task = Task::generate(&Spec::new("fn.sum", 3, 5)).unwrap();
    let (cell, reference) = task
        .checks
        .iter()
        .find_map(|c| match &c.kind {
            Kind::Formula { cell, reference, .. } => Some((cell.clone(), reference.clone())),
            _ => None,
        })
        .unwrap();
    app.load(task, None, None, None).unwrap();
    app.screen = Screen::Lesson;
    app.sheet.session.execute("selection.set", json!({"range": cell})).unwrap();
    let mut h = harness(app);
    let typed = ru(&reference);
    assert!(typed.contains("СУММ("), "{typed}");
    h.input_mut().events.push(egui::Event::Text(typed.clone()));
    h.run_steps(3);
    h.input_mut().events.push(key(egui::Key::Enter, egui::Modifiers::NONE));
    h.run_steps(4);
    let fb = h.state().feedback.as_ref().expect("graded after Enter");
    assert!(fb.passed, "typed {typed}: {:?}", fb.eval.outcomes);
}

#[test]
fn shortcuts_are_recorded_by_physical_key() {
    let mut app = LearningApp::new(None).unwrap();
    app.screen = Screen::Lesson;
    let mut raw = egui::RawInput {
        events: vec![
            key(egui::Key::L, egui::Modifiers::CTRL | egui::Modifiers::SHIFT),
            key(egui::Key::F4, egui::Modifiers::NONE),
            key(egui::Key::A, egui::Modifiers::NONE),
            egui::Event::Key {
                key: egui::Key::Equals,
                physical_key: Some(egui::Key::Equals),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::ALT,
            },
        ],
        ..Default::default()
    };
    app.raw_input_hook(&mut raw);
    assert_eq!(app.keys, vec!["Ctrl+Shift+L", "F4", "Alt+="]);
}

#[test]
fn progress_and_the_open_exercise_survive_a_restart() {
    let dir = std::env::temp_dir().join(format!("gridcraft-learn-test-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("progress.json");
    let _ = std::fs::remove_file(&path);
    let mut app = LearningApp::new(Some(path.clone())).unwrap();
    app.start_lesson(None).unwrap();
    let spec = app.active.as_ref().unwrap().task.spec.clone();
    solve(&mut app);
    app.check().unwrap();
    app.advance().unwrap();
    let next = app.active.as_ref().unwrap().task.spec.clone();
    app.sheet.session.execute("cell.set", json!({"cell": "Z1", "value": "черновик"})).unwrap();
    app.save().unwrap();
    let xp = app.profile.xp;
    assert!(xp > 0);
    drop(app);
    let back = LearningApp::new(Some(path.clone())).unwrap();
    assert_eq!(back.profile.xp, xp);
    assert_eq!(back.screen, Screen::Lesson);
    assert_eq!(back.active.as_ref().unwrap().task.spec, next);
    assert_ne!(spec, next);
    assert_eq!(gridcraft_learn_core::sheet::value(&back.sheet.session, "Z1").unwrap(), gridcraft_engine::core::Value::Text("черновик".into()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_interview_starts_without_hints_and_grades_on_submit() {
    let mut app = LearningApp::new(None).unwrap();
    for skill in CASES[0].requires {
        app.profile.skills.entry((*skill).into()).or_default().level = 2;
    }
    app.start_interview().unwrap();
    assert_eq!(app.screen, Screen::Case);
    let a = app.active.as_ref().unwrap();
    assert!(a.interview.is_some());
    assert!(a.task.hints.is_empty());
    app.submit_case(false).unwrap();
    assert_eq!(app.screen, Screen::Report);
    let r = app.report.as_ref().unwrap();
    assert!(r.eval.score < 0.5, "an untouched sheet must not pass");
    assert!(!app.profile.remediation.is_empty());
}

#[test]
fn every_screen_renders() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_case("case.payroll", false).unwrap();
    let mut h = harness(app);
    for screen in [Screen::Home, Screen::Lesson, Screen::Case, Screen::Done, Screen::Report, Screen::Stats] {
        h.state_mut().screen = screen;
        h.run_steps(3);
    }
    h.state_mut().show_settings = true;
    h.run_steps(3);
}

/// All the JSONL lines written under `progress.json`'s directory.
fn read_events(progress: &Path) -> String {
    let events = progress.with_file_name("events");
    let Ok(dir) = std::fs::read_dir(&events) else { return String::new() };
    dir.flatten().map(|e| std::fs::read_to_string(e.path()).unwrap_or_default()).collect()
}

fn log_bytes(progress: &Path) -> u64 {
    let events = progress.with_file_name("events");
    std::fs::read_dir(&events).map(|dir| dir.flatten().filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum()).unwrap_or(0)
}

#[test]
fn action_log_records_domain_events_and_toggles_off() {
    let dir = supply_test_dir("events-app-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("progress.json");
    let mut app = LearningApp::new(Some(path.clone())).unwrap();
    app.start_lesson(None).unwrap();
    solve(&mut app);
    app.check().unwrap();
    app.advance().unwrap();
    app.save().unwrap();

    let text = read_events(&path);
    for kind in ["\"task.start\"", "\"check\"", "\"advance\"", "\"save\""] {
        assert!(text.contains(kind), "missing {kind} in:\n{text}");
    }
    assert!(text.contains("\"skill\""), "task.start carries the skill:\n{text}");
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(v["ts"].as_u64().unwrap() > 0);
        assert!(v["session"].as_u64().unwrap() > 0);
        assert!(v["kind"].is_string());
        assert!(v["screen"].is_string());
    }

    // Off-switch: a disabled log never grows.
    let before = log_bytes(&path);
    app.profile.logging = false;
    app.events.set_enabled(false);
    app.save().unwrap();
    assert_eq!(before, log_bytes(&path));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn raw_input_is_logged_with_its_modality() {
    let dir = supply_test_dir("events-input-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("progress.json");
    let mut app = LearningApp::new(Some(path.clone())).unwrap();
    app.start_lesson(None).unwrap();
    app.sheet.session.execute("selection.set", json!({"range": "B2"})).unwrap();
    let mut raw = egui::RawInput {
        events: vec![
            egui::Event::Text("=СУММ(".into()),
            key(egui::Key::L, egui::Modifiers::CTRL),
            egui::Event::PointerButton {
                pos: egui::pos2(10.0, 20.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerMoved(egui::pos2(11.0, 21.0)),
            egui::Event::Paste("x".into()),
            egui::Event::Copy,
        ],
        ..Default::default()
    };
    app.raw_input_hook(&mut raw);
    let text = read_events(&path);
    for needle in [
        "\"text\"",
        "\"modality\":\"keyboard\"",
        "\"paste\"",
        "\"modality\":\"clipboard\"",
        "\"pointer\"",
        "\"drag\"",
        "\"modality\":\"mouse\"",
        "\"key\"",
        "\"cell\":\"B2\"",
    ] {
        assert!(text.contains(needle), "missing {needle} in:\n{text}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn malformed_formula_is_logged_as_an_issue() {
    let dir = supply_test_dir("events-issue-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("progress.json");
    let mut app = LearningApp::new(Some(path.clone())).unwrap();
    let task = Task::generate(&Spec::new("fn.sum", 2, 5)).unwrap();
    let cell = task
        .checks
        .iter()
        .find_map(|c| match &c.kind {
            Kind::Formula { cell, .. } => Some(cell.clone()),
            _ => None,
        })
        .unwrap();
    app.load(task, None, None, None).unwrap();
    // Inject a malformed formula the engine keeps verbatim but cannot parse.
    {
        let at = gridcraft_engine::core::CellRef::parse(&cell).unwrap();
        let f = gridcraft_engine::model::Formula::new("IF(B2>C2(=IF(D2>2));\"Нет\")");
        let doc = app.sheet.session.doc_mut().unwrap();
        let wb = std::sync::Arc::make_mut(&mut doc.wb);
        let idx = wb.active_sheet;
        let style = wb.sheet(idx).and_then(|s| s.cell(at)).map(|c| c.style).unwrap_or_default();
        wb.sheet_mut(idx).unwrap().set_cell(
            at,
            gridcraft_engine::model::Cell { value: gridcraft_engine::core::Value::Empty, formula: Some(std::sync::Arc::new(f)), style },
        );
    }
    app.check().unwrap();
    let text = read_events(&path);
    assert!(text.contains("\"issues\""), "check event carries issues:\n{text}");
    assert!(text.contains("CallInsideOperand"), "malformed formula is classified:\n{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn zoom_interface_scale_and_logging_survive_a_restart() {
    let dir = supply_test_dir("zoom-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("progress.json");
    let mut app = LearningApp::new(Some(path.clone())).unwrap();
    app.start_lesson(None).unwrap();
    app.sheet.ui.interface_scale = 150;
    app.sheet.session.execute("view.zoom", json!({"percent": 175})).unwrap();
    app.profile.logging = false;
    app.events.set_enabled(false);
    app.save().unwrap();

    let back = LearningApp::new(Some(path.clone())).unwrap();
    assert_eq!(back.profile.interface_scale, 150);
    assert_eq!(back.sheet.ui.interface_scale, 150);
    assert_eq!(back.profile.sheet_zoom, 175);
    assert!(!back.profile.logging);
    assert!(!back.events.is_enabled());
    let doc = back.sheet.session.doc().unwrap();
    assert_eq!(doc.wb.sheets[doc.wb.active_sheet].zoom, 175);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sheet_zoom_survives_a_step_change() {
    let mut app = LearningApp::new(None).unwrap();
    app.profile.sheet_zoom = 175;
    let task = Task::generate(&Spec::new("fn.sum", 3, 5)).unwrap();
    app.load(task, None, None, None).unwrap();
    let doc = app.sheet.session.doc().unwrap();
    assert_eq!(doc.wb.sheets[doc.wb.active_sheet].zoom, 175);
}

#[test]
fn course_cursor_persists_across_a_restart() {
    let dir = supply_test_dir("cursor-persist-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("progress.json");
    let mut app = LearningApp::new(Some(path.clone())).unwrap();
    app.start_lesson(None).unwrap();
    let skill = app.active.as_ref().unwrap().task.spec.skill.clone();
    assert_eq!(app.profile.cursor.as_deref(), Some(skill.as_str()), "loading a task sets the cursor");
    app.save().unwrap();

    let back = LearningApp::new(Some(path.clone())).unwrap();
    assert_eq!(back.profile.cursor.as_deref(), Some(skill.as_str()));
    assert_eq!(back.active.as_ref().unwrap().task.spec.skill, skill);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cursor_migrates_from_current_and_resumes_supply_at_its_stage() {
    let dir = supply_test_dir("cursor-migration-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("progress.json");
    let mut app = LearningApp::new(Some(path.clone())).unwrap();
    app.start_supply().unwrap();
    for _ in 0..3 {
        solve(&mut app);
        app.check_supply(false).unwrap();
    }
    assert_eq!(app.supply_stage(), Some(3));
    app.save().unwrap();

    // An older file that predates `cursor`: strip the field and reload.
    let bytes = std::fs::read(&path).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["profile"].as_object_mut().unwrap().remove("cursor");
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();

    let restored = LearningApp::new(Some(path.clone())).unwrap();
    assert_eq!(restored.profile.cursor.as_deref(), Some("route.supply.3"), "migrated from `current`");
    assert_eq!(restored.screen, Screen::Supply);
    assert_eq!(restored.supply_stage(), Some(3));
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn continue_course_with_a_skill_cursor_starts_the_frontier_lesson() {
    let mut app = LearningApp::new(None).unwrap();
    app.profile.set_cursor("fn.sum");
    app.continue_course();
    assert_eq!(app.screen, Screen::Lesson);
    let skill = app.active.as_ref().expect("a lesson started").task.spec.skill.clone();
    assert_eq!(Some(skill.as_str()), app.profile.frontier(), "the autopilot starts at the frontier");
}

#[test]
fn continue_course_goes_to_the_frontier_while_supply_resumes_on_its_own_link() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_supply().unwrap();
    for _ in 0..5 {
        solve(&mut app);
        app.check_supply(false).unwrap();
    }
    assert_eq!(app.supply_stage(), Some(5));
    assert_eq!(app.profile.cursor.as_deref(), Some("route.supply.5"));

    // Leaving the screen keeps the position in the profile; the exercise itself is dropped.
    app.quit_to_home();
    assert!(app.active.is_none());
    assert_eq!(app.profile.cursor.as_deref(), Some("route.supply.5"));

    // The primary "Продолжить" follows the skill autopilot, not the route section.
    app.continue_course();
    assert_eq!(app.screen, Screen::Lesson, "the one course continues by skill, not the supply basics; screen={:?}", app.screen);

    // The supply section is entered deliberately and resumes at the saved stage, not stage 0.
    app.open_supply().unwrap();
    assert_eq!(app.screen, Screen::Supply);
    assert_eq!(app.supply_stage(), Some(5), "the route resumes at its saved stage");
    assert_eq!(app.profile.cursor.as_deref(), Some("route.supply.5"));
}

#[test]
fn a_passed_drill_continues_on_live_data_below_it_in_the_same_book() {
    let mut app = LearningApp::new(None).unwrap();
    // The learner does СУММЕСЛИ with support; this fast clean pass lifts it to «Сам» (level 3),
    // which is when live data starts.
    app.profile.skills.entry("cond.sumif".into()).or_default().level = 2;
    app.start_lesson(Some("cond.sumif")).unwrap();
    let drill = app.active.as_ref().unwrap().task.clone();
    assert_eq!(drill.spec.skill, "cond.sumif");
    solve(&mut app);
    app.check().unwrap();
    assert!(app.feedback.as_ref().is_some_and(|f| f.passed));
    let attempts = app.profile.skills.get("cond.sumif").unwrap().attempts;
    app.advance().unwrap();

    let live = app.active.as_ref().unwrap().task.clone();
    assert_eq!(live.spec.skill, "live.cond.sumif");
    assert!(matches!(app.active.as_ref().unwrap().slot, Some(Slot::Live(_))));
    // Same book: the solved drill is still there, and the register opens below it.
    for (cell, _) in formulas(&drill) {
        assert!(gridcraft_learn_core::sheet::has_formula(&app.sheet.session.doc().unwrap().wb, &cell), "{cell} was cleared");
    }
    let drill_last = drill.setup.cells.iter().filter_map(|(c, ..)| gridcraft_engine::core::CellRef::parse(c)).map(|a| a.row).max().unwrap();
    let live_first = live.setup.cells.iter().filter_map(|(c, ..)| gridcraft_engine::core::CellRef::parse(c)).map(|a| a.row).min().unwrap();
    assert!(live_first > drill_last + 1);
    // The view shows the new register: its title row is not above the top edge.
    let title_y = app.sheet.session.doc().unwrap().wb.sheet(0).unwrap().row_top(live_first) as f32;
    assert!(app.sheet.view().scroll.y <= title_y, "the view follows the new register");
    assert!(app.feedback.is_none());

    // A wrong answer names a trap instead of a bare "wrong", and the book stays as it is.
    let Kind::Live { cell, expected, .. } = live.checks[0].kind.clone() else { panic!("live check") };
    let cell = cell.expect("tier 1 answer cell");
    let _ = app.sheet.session.execute("cell.set", json!({"cell": cell, "input": "=1+A1"}));
    app.check().unwrap();
    let fb = app.feedback.as_ref().unwrap();
    assert!(!fb.passed);
    assert!(fb.eval.outcomes[0].feedback.contains("ошибки ввода") || fb.eval.outcomes[0].feedback.contains("не учтено"));
    app.fix();
    let _ = app.sheet.session.execute("cell.set", json!({"cell": cell, "input": format!("=0*ROWS(A1:A2)+{expected}")}));
    app.check().unwrap();
    assert!(app.feedback.as_ref().unwrap().passed);
    // Live data moves its own dirt tier, never the clean-drill staircase.
    assert_eq!(app.profile.skills.get("cond.sumif").unwrap().attempts, attempts);
    assert_eq!(app.profile.live.get("cond.sumif").unwrap().attempts, 1);
    assert!(app.profile.evidence.get("cond.sumif").unwrap().transfer.passed + app.profile.evidence.get("cond.sumif").unwrap().transfer.failed >= 1);

    // Once per skill per lesson: the next drill opens a fresh book.
    app.advance().unwrap();
    let next = app.active.as_ref().unwrap().task.clone();
    assert_eq!(next.spec.skill, "cond.sumif");
    solve(&mut app);
    app.check().unwrap();
    app.advance().unwrap();
    assert_ne!(app.active.as_ref().unwrap().task.spec.skill, "live.cond.sumif");
}

#[test]
fn a_missed_register_returns_later_as_a_fresh_register() {
    let mut app = LearningApp::new(None).unwrap();
    app.profile.skills.entry("fn.sum".into()).or_default().level = 2;
    app.start_lesson(Some("fn.sum")).unwrap();
    solve(&mut app);
    app.check().unwrap();
    app.advance().unwrap();
    assert_eq!(app.active.as_ref().unwrap().task.spec.skill, "live.fn.sum");
    app.check().unwrap();
    assert!(!app.feedback.as_ref().unwrap().passed);
    app.advance().unwrap();
    let lesson = app.active.as_ref().unwrap().lesson.clone().unwrap();
    assert!(matches!(lesson.queue.back(), Some(Slot::Live(s)) if s == "fn.sum"));
}

#[test]
fn the_live_step_renders_with_its_tag() {
    let mut app = LearningApp::new(None).unwrap();
    app.profile.skills.entry("lookup.vlookup".into()).or_default().level = 3;
    app.start_lesson(Some("lookup.vlookup")).unwrap();
    solve(&mut app);
    app.check().unwrap();
    app.advance().unwrap();
    assert_eq!(app.active.as_ref().unwrap().task.spec.skill, "live.lookup.vlookup");
    let mut h = harness(app);
    h.run_steps(3);
    use egui_kittest::kittest::Queryable;
    assert!(h.query_by_label_contains("Андрей, снабжение").is_some());
    assert!(h.query_by_label_contains("Данные можно чистить").is_some());
}

#[test]
fn a_known_skill_is_reviewed_straight_on_live_data() {
    let mut app = LearningApp::new(None).unwrap();
    let p = app.profile.skills.entry("cond.sumif".into()).or_default();
    p.level = 3;
    p.card.reps = 2;
    p.card.due = 1;
    app.start_lesson(None).unwrap();
    let lesson = app.active.as_ref().unwrap();
    assert!(matches!(&lesson.slot, Some(Slot::Live(s)) if s == "cond.sumif"), "{:?}", lesson.slot);
    assert_eq!(lesson.task.spec.skill, "live.cond.sumif");
    // Passing it on live data answers the due review.
    let Kind::Live { cell, expected, .. } = lesson.task.checks[0].kind.clone() else { panic!("live") };
    let _ = app.sheet.session.execute("cell.set", json!({"cell": cell.unwrap(), "input": format!("=0*ROWS(A1:A2)+{expected}")}));
    app.check().unwrap();
    assert!(app.feedback.as_ref().unwrap().passed);
    assert!(app.profile.skills.get("cond.sumif").unwrap().card.due > 1);
}

#[test]
fn report_assembly_opens_on_the_report_sheet_with_sources_beside_it() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_lesson(Some("asm.report")).unwrap();
    let wb = app.sheet.session.doc().unwrap().wb.clone();
    let names: Vec<&str> = wb.sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names.first(), Some(&"Отчёт"));
    assert!(names.contains(&"Выгрузка 1С") && names.contains(&"От Марата"), "{names:?}");
    assert_eq!(wb.active_sheet, 0);
    // Solved through the real session: the report check passes and the lesson moves on.
    solve_table(&mut app);
    app.check().unwrap();
    assert!(app.feedback.as_ref().unwrap().passed, "{:?}", app.feedback.as_ref().unwrap().eval.outcomes);
}

fn solve_table(app: &mut LearningApp) {
    let task = app.active.as_ref().unwrap().task.clone();
    for c in &task.checks {
        if let Kind::Table { first, left, cols, rows, .. } = &c.kind {
            for (i, row) in rows.iter().enumerate() {
                for (k, want) in row.iter().enumerate() {
                    if cols[k] == gridcraft_learn_core::check::Col::Given {
                        continue;
                    }
                    let cell = gridcraft_learn_core::sheet::a1(first + i as u32, left + k as u32);
                    let params = match want {
                        gridcraft_learn_core::Expectation::Number(n) => json!({"cell": cell, "value": n}),
                        gridcraft_learn_core::Expectation::Text(t) => json!({"cell": cell, "value": t}),
                        _ => continue,
                    };
                    app.sheet.session.execute("cell.set", params).unwrap();
                }
            }
        }
    }
}

#[test]
#[ignore = "writes screenshots for a human check"]
fn zz_asm_screenshots() {
    for (skill, level, sheet, name) in
        [("asm.report", 3u8, 0usize, "report-l3"), ("asm.report", 3, 2, "report-l3-marat"), ("asm.dates", 2, 1, "dates-l2-src"), ("asm.pivot", 1, 0, "pivot-l1"), ("asm.pivot", 3, 2, "pivot-l3-feb")]
    {
        let mut app = LearningApp::new(None).unwrap();
        app.profile.skills.entry(skill.into()).or_default().level = level;
        app.start_lesson(Some(skill)).unwrap();
        std::sync::Arc::make_mut(&mut app.sheet.session.doc_mut().unwrap().wb).active_sheet = sheet;
        let mut h = Harness::builder().with_size(egui::vec2(1500.0, 900.0)).with_max_steps(20).build_ui_state(
            |ui, app: &mut LearningApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.run_steps(8);
        h.render().expect("render").save(format!("/tmp/asm-{name}.png")).expect("save");
    }
}

#[test]
#[ignore = "writes screenshots for a human check"]
fn zz_live_screenshots() {
    for (skill, tier, name) in [("cond.sumif", 1u8, "sumif-t1"), ("cond.sumifs", 3, "sumifs-t3"), ("lookup.vlookup", 2, "vlookup-t2")] {
        let mut app = LearningApp::new(None).unwrap();
        app.profile.skills.entry(skill.into()).or_default().level = 3;
        app.profile.live.entry(skill.into()).or_default().tier = tier;
        app.start_lesson(Some(skill)).unwrap();
        solve(&mut app);
        app.check().unwrap();
        app.advance().unwrap();
        let mut h = Harness::builder().with_size(egui::vec2(1500.0, 900.0)).with_max_steps(20).build_ui_state(
            |ui, app: &mut LearningApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            app,
        );
        h.run_steps(8);
        let img = h.render().expect("render");
        img.save(format!("/tmp/live-{name}.png")).expect("save");
    }
}

#[test]
fn wrong_answers_and_their_fixes_go_to_the_mistakes_journal() {
    let dir = std::env::temp_dir().join(format!("gridcraft-learn-mistakes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("progress.json");
    let mut app = LearningApp::new(Some(path)).unwrap();
    app.start_lesson(Some("cond.sumif")).unwrap();
    let cell = app.active.as_ref().unwrap().task.answer_cells()[0].clone();
    app.sheet.session.execute("cell.set", json!({"cell": cell, "value": "=1+1"})).unwrap();
    app.check().unwrap();
    solve(&mut app);
    app.check().unwrap();
    let text = std::fs::read_to_string(dir.join("mistakes.jsonl")).expect("journal written");
    let lines: Vec<serde_json::Value> = text.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(lines[0]["result"], "wrong");
    assert_eq!(lines[0]["skill"], "cond.sumif");
    assert_eq!(lines[0]["typed"][0]["typed"], "=1+1");
    assert!(lines[0]["wrong"][0]["said"].as_str().is_some_and(|s| !s.is_empty()));
    assert_eq!(lines[1]["result"], "fixed");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn start_over_puts_a_scrambled_table_back() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_lesson(Some("data.sort")).unwrap();
    let task = app.active.as_ref().unwrap().task.clone();
    let gridcraft_learn_core::check::Kind::Sorted { first, last, .. } = task.checks[0].kind.clone() else { panic!() };
    // Sort only the first column without expanding: the rows are torn apart.
    app.sheet.session.execute("selection.set", json!({"range": format!("A{first}:A{last}")})).unwrap();
    app.sheet.session.execute("data.sort", json!({"expand": false, "keys": [{"column": "A", "order": "desc"}]})).unwrap();
    let before = gridcraft_learn_core::sheet::value(&task.session().unwrap(), &format!("A{first}")).unwrap();
    app.reset_task().unwrap();
    assert_eq!(gridcraft_learn_core::sheet::value(&app.sheet.session, &format!("A{first}")).unwrap(), before);
    assert!(app.feedback.is_none());
}

#[test]
fn live_data_waits_for_level_three_and_rests_after_two_misses() {
    // A pass that leaves the skill at level 2 does not open live data yet.
    let mut app = LearningApp::new(None).unwrap();
    app.profile.skills.entry("cond.sumif".into()).or_default().level = 1;
    app.start_lesson(Some("cond.sumif")).unwrap();
    solve(&mut app);
    app.check().unwrap();
    assert!(app.profile.level("cond.sumif") < 3);
    app.advance().unwrap();
    assert!(!app.active.as_ref().is_some_and(|a| a.task.spec.skill.starts_with("live.")), "live data before «Сам»");

    // Two misses in a row on live data: walkthrough opened, register rests, no loop.
    let mut app = LearningApp::new(None).unwrap();
    app.profile.skills.entry("cond.sumif".into()).or_default().level = 3;
    for round in 0..2 {
        let spec = gridcraft_learn_core::content::live::spec("cond.sumif", 1, 7 + round, 1);
        let task = gridcraft_learn_core::Task::generate(&spec).unwrap();
        app.start_lesson(Some("cond.sumif")).unwrap();
        app.sheet.session = task.session().unwrap();
        if let Some(a) = app.active.as_mut() {
            a.task = task.clone();
            a.slot = Some(Slot::Live("cond.sumif".into()));
        }
        let Kind::Live { cell, .. } = task.checks[0].kind.clone() else { panic!() };
        let _ = app.sheet.session.execute("cell.set", json!({"cell": cell.unwrap(), "input": "=1+A1"}));
        app.check().unwrap();
    }
    let fb = app.feedback.as_ref().unwrap();
    assert!(fb.change.live_rest && fb.note.is_some());
    assert_eq!(app.active.as_ref().unwrap().hints as usize, app.active.as_ref().unwrap().task.hints.len(), "walkthrough opened");
    assert!(!app.profile.live_ready("cond.sumif", crate::now()));
    app.advance().unwrap();
    let lesson = app.active.as_ref().and_then(|a| a.lesson.as_ref());
    assert!(lesson.is_none_or(|l| !l.queue.iter().any(|s| matches!(s, Slot::Live(_)))), "a resting register is not re-queued");
}

#[test]
fn disputing_a_miss_undoes_its_penalty_and_logs_the_case() {
    let dir = std::env::temp_dir().join(format!("gridcraft-learn-dispute-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    let mut app = LearningApp::new(Some(dir.join("progress.json"))).unwrap();
    app.profile.skills.entry("cond.sumif".into()).or_default().level = 3;
    app.profile.skills.entry("cond.sumif".into()).or_default().clean = 2;
    app.start_lesson(Some("cond.sumif")).unwrap();
    let cell = app.active.as_ref().unwrap().task.answer_cells()[0].clone();
    app.sheet.session.execute("cell.set", json!({"cell": cell, "value": "=1+1"})).unwrap();
    app.check().unwrap();
    assert_eq!(app.profile.skills["cond.sumif"].clean, 0, "the miss reset the streak");
    app.dispute().unwrap();
    assert_eq!(app.profile.skills["cond.sumif"].clean, 2, "a disputed miss costs nothing");
    assert_eq!(app.profile.skills["cond.sumif"].level, 3);
    let text = std::fs::read_to_string(dir.join("mistakes.jsonl")).unwrap();
    assert!(text.lines().any(|l| l.contains("\"result\":\"disputed\"")), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_skip_is_not_a_lapse_and_the_skill_waits_for_the_next_lesson() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_lesson(Some("data.sort")).unwrap();
    assert_eq!(app.active.as_ref().unwrap().task.spec.skill, "data.sort");
    let before = app.profile.skills.get("data.sort").map(|p| (p.level, p.misses, p.card.lapses, p.attempts));
    for _ in 0..6 {
        app.advance().unwrap();
        assert!(!app.active.as_ref().is_some_and(|a| a.task.spec.skill == "data.sort"), "a skipped skill came back in the same lesson");
    }
    let after = app.profile.skills.get("data.sort").map(|p| (p.level, p.misses, p.card.lapses, p.attempts));
    assert_eq!(before, after, "skipping must not grade the skill");
}



