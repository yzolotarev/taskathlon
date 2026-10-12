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

#[test]
fn personalized_zoom_survives_exercises_and_restart_without_an_open_task() {
    let dir = std::env::temp_dir().join(format!("taskathlon-zoom-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("progress.json");
    let _ = std::fs::remove_file(&path);
    let mut app = LearningApp::new(Some(path.clone())).unwrap();
    app.start_lesson(None).unwrap();
    app.sheet.session.run("view.zoom", json!({"percent": 160})).unwrap();
    app.sheet.ui.interface_scale = 125;
    // Moving on before autosave must still carry the user's current selection.
    solve(&mut app);
    app.check().unwrap();
    app.advance().unwrap();
    assert_eq!(app.sheet.session.active().unwrap().wb.active().unwrap().zoom, 160);
    app.save().unwrap();
    let mut resumed = LearningApp::new(Some(path.clone())).unwrap();
    assert_eq!(resumed.sheet.session.active().unwrap().wb.active().unwrap().zoom, 160);
    assert_eq!(resumed.sheet.ui.interface_scale, 125);
    // Finishing/leaving the worksheet must retain the preference separately from its data.
    resumed.active = None;
    resumed.save().unwrap();
    let mut back = LearningApp::new(Some(path)).unwrap();
    assert!(back.active.is_none());
    back.start_lesson(None).unwrap();
    assert_eq!(back.sheet.session.active().unwrap().wb.active().unwrap().zoom, 160);
    assert_eq!(back.sheet.ui.interface_scale, 125);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn legacy_profiles_get_default_zoom_without_resetting_progress() {
    let profile: Profile = serde_json::from_value(json!({"xp": 321})).unwrap();
    assert_eq!(profile.xp, 321);
    assert_eq!(profile.sheet_zoom, 100);
    assert_eq!(profile.interface_scale, 100);
}

#[test]
fn formula_error_explains_once_and_practice_survives_skipping() {
    let mut app = LearningApp::new(None).unwrap();
    app.start_lesson(Some("fn.sum")).unwrap();
    let cell = app.active.as_ref().unwrap().task.answer_cells()[0].clone();
    gridcraft_learn_core::sheet::set(&mut app.sheet.session, &cell, &Cell::Formula("=NO_SUCH_FUNCTION(1)".into())).unwrap();
    app.observe_errors();
    assert_eq!(app.error_encounters.len(), 1);
    assert_eq!(app.profile.pending_errors, vec!["#NAME?"]);
    app.observe_errors();
    assert_eq!(app.error_encounters.len(), 1);
    assert_eq!(app.active.as_ref().unwrap().hints, 1);
    assert_eq!(app.show_hints, 0);
    app.hint();
    assert_eq!(app.show_hints, 1);
    assert_eq!(app.active.as_ref().unwrap().hints, 2);
    app.advance().unwrap();
    assert!(app.active.as_ref().unwrap().task.spec.skill.starts_with("errors."));
    assert_eq!(app.profile.pending_errors, vec!["#NAME?"]);
    app.advance().unwrap();
    assert_eq!(app.profile.pending_errors, vec!["#NAME?"]);
    app.quit_to_home();
    app.start_lesson(Some("fn.sum")).unwrap();
    assert!(app.active.as_ref().unwrap().task.spec.skill.starts_with("errors."));
    solve(&mut app);
    app.check().unwrap();
    assert!(app.profile.pending_errors.is_empty());
}

#[test]
fn testout_keeps_pending_error_practice_out_until_assessment_finishes() {
    let mut app = LearningApp::new(None).unwrap();
    app.profile.pending_errors.push("#NAME?".into());
    app.start_testout(0).unwrap();
    let tasks = app.active.as_ref().unwrap().lesson.as_ref().unwrap().total;
    for _ in 0..tasks {
        assert!(matches!(app.active.as_ref().unwrap().slot, Some(Slot::TestOut(_))));
        solve(&mut app);
        app.check().unwrap();
        app.advance().unwrap();
    }
    assert_eq!(app.screen, Screen::Done);
    assert_eq!(app.profile.pending_errors, vec!["#NAME?"]);
}

#[test]
fn restarted_final_testout_preserves_slot_and_suppresses_error_help() {
    let path = std::env::temp_dir().join(format!("gridcraft-final-assessment-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut app = LearningApp::new(Some(path.clone())).unwrap();
    app.start_testout(0).unwrap();
    app.active.as_mut().unwrap().lesson.as_mut().unwrap().queue.clear();
    let slot = app.active.as_ref().unwrap().slot.clone();
    app.save().unwrap();
    let mut resumed = LearningApp::new(Some(path.clone())).unwrap();
    assert_eq!(resumed.active.as_ref().unwrap().slot, slot);
    assert!(resumed.active.as_ref().unwrap().lesson.as_ref().unwrap().test_out);
    let cell = resumed.active.as_ref().unwrap().task.answer_cells()[0].clone();
    gridcraft_learn_core::sheet::set(&mut resumed.sheet.session, &cell, &Cell::Formula("=NO_SUCH_FUNCTION(1)".into())).unwrap();
    resumed.observe_errors();
    assert!(resumed.error_encounters.is_empty());
    assert!(resumed.profile.seen_errors.is_empty());
    let _ = std::fs::remove_file(path);
}

#[test]
fn later_error_reviews_keep_the_diagnosis_question() {
    let mut app = LearningApp::new(None).unwrap();
    let skill = gridcraft_learn_core::errors::skill_id("#NAME?");
    let task = Task::generate(&Spec::new(&skill, 3, 5)).unwrap();
    app.load(task, None, None, Some(Slot::Review(skill))).unwrap();
    assert!(app.active.as_ref().unwrap().choosing().is_some());
}

#[test]
fn mentor_assistance_does_not_reveal_task_hints_after_restart() {
    let path = std::env::temp_dir().join(format!("gridcraft-mentor-hints-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut app = LearningApp::new(Some(path.clone())).unwrap();
    app.start_lesson(Some("fn.sum")).unwrap();
    let cell = app.active.as_ref().unwrap().task.answer_cells()[0].clone();
    gridcraft_learn_core::sheet::set(&mut app.sheet.session, &cell, &Cell::Formula("=NO_SUCH_FUNCTION(1)".into())).unwrap();
    app.observe_errors();
    app.save().unwrap();
    let mut resumed = LearningApp::new(Some(path.clone())).unwrap();
    assert_eq!(resumed.show_hints, 0);
    assert_eq!(resumed.active.as_ref().unwrap().hints, 1);
    resumed.hint();
    assert_eq!(resumed.show_hints, 1);
    assert_eq!(resumed.active.as_ref().unwrap().hints, 2);
    resumed.save().unwrap();
    let back = LearningApp::new(Some(path.clone())).unwrap();
    assert_eq!(back.show_hints, 1);
    assert_eq!(back.active.as_ref().unwrap().hints, 2);
    let _ = std::fs::remove_file(path);
}
