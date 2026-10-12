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
