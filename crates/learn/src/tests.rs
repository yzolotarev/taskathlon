#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::check::Kind;
use crate::content::{CASES, SKILLS};
use crate::profile::DAY;
use crate::sheet::{self, Cell};
use crate::*;
use gridcraft_engine::Session;
use serde_json::json;

#[test]
fn shown_example_cannot_fast_track_independent_mastery() {
    let mut profile = Profile::default();
    let task = Task::generate(&Spec::new("start.mul", 1, 42)).unwrap();
    assert!(task.example.is_some());
    let first = profile.record(&task, true, 1.0, 0, 0, false, 100);
    assert!(!first.fast_track);
    assert_eq!(first.speed_bonus, 0);
    assert_eq!(profile.level("start.mul"), 1);
    profile.record(&task, true, 1.0, 0, 0, false, 101);
    assert_eq!(profile.level("start.mul"), 2);
}

#[test]
fn profile_without_cursor_loads_and_cursor_round_trips() {
    let mut profile = Profile::default();
    assert!(profile.cursor.is_none());
    assert!(profile.supply_stage.is_none());
    profile.set_cursor("lookup.vlookup");
    assert_eq!(profile.cursor.as_deref(), Some("lookup.vlookup"));
    let encoded = serde_json::to_string(&profile).unwrap();
    let back: Profile = serde_json::from_str(&encoded).unwrap();
    assert_eq!(back.cursor.as_deref(), Some("lookup.vlookup"));
    // A profile saved before the field existed still decodes (serde default).
    let legacy: Profile = serde_json::from_str("{\"xp\":10}").unwrap();
    assert!(legacy.cursor.is_none());
    assert_eq!(legacy.xp, 10);
}

#[test]
fn lesson_keeps_remaining_remediation_and_does_not_cross_domains() {
    let mut profile =
        Profile { remediation: vec!["start.add".into(), "start.sub".into(), "start.mul".into(), "sql.select".into()], ..Profile::default() };
    let lesson = profile.plan_lesson(100, Some("start.add"));
    assert!(profile.remediation.contains(&"start.mul".to_string()));
    assert!(profile.remediation.contains(&"sql.select".to_string()));
    for slot in lesson.queue {
        if let Slot::Review(id) | Slot::Mix(id) | Slot::Retry(id) = slot {
            assert!(!id.starts_with("sql."));
        }
    }
}

/// Plays the reference solution for every check of a task. Returns the keys "pressed".
fn solve(task: &Task, s: &mut Session) -> Vec<String> {
    let mut keys = Vec::new();
    let mut pair_row = 1;
    for c in &task.checks {
        match &c.kind {
            Kind::Expected { cell, expected, formula } => {
                if *formula && sheet::has_formula(&s.doc().unwrap().wb, cell) {
                    continue;
                }
                let value = match expected {
                    crate::check::Expectation::Number(n) => Cell::Num(*n),
                    crate::check::Expectation::Text(t) => Cell::Text(t.clone()),
                    crate::check::Expectation::Bool(b) => Cell::Formula(format!("={}", if *b { "TRUE()" } else { "FALSE()" })),
                    crate::check::Expectation::Empty => Cell::Text(String::new()),
                    crate::check::Expectation::Error(_) => continue,
                };
                let value = if *formula {
                    match value {
                        Cell::Num(n) => Cell::Formula(format!("={n}")),
                        other => other,
                    }
                } else {
                    value
                };
                sheet::set(s, cell, &value).unwrap();
            }
            Kind::Selection { range } => {
                s.execute("selection.set", json!({"range": range})).unwrap();
            }
            // Live registers are solved by their own module tests (cleaning, then a formula).
            Kind::Live { .. } => {}
            Kind::Table { first, left, cols, rows, .. } => {
                for (i, row) in rows.iter().enumerate() {
                    for (c, want) in row.iter().enumerate() {
                        if cols.get(c) == Some(&crate::check::Col::Given) {
                            continue;
                        }
                        let value = match want {
                            crate::check::Expectation::Number(n) => Cell::Num(*n),
                            crate::check::Expectation::Text(t) => Cell::Text(t.clone()),
                            _ => continue,
                        };
                        sheet::set(s, &sheet::a1(first + i as u32, left + c as u32), &value).unwrap();
                    }
                }
            }
            Kind::Pivot { sheet, source, rows, columns, values } => {
                let values: Vec<_> = values.iter().map(|(field, func)| json!({"field": field, "func": format!("{func:?}").to_lowercase()})).collect();
                // No sheet: the learner's path, Insert → PivotTable onto a new sheet in front.
                let destination = if sheet.is_empty() { "new".to_string() } else { format!("{sheet}!AZ100") };
                s.execute(
                    "insert.pivotTable",
                    json!({"source": source, "destination": destination, "rows": rows, "columns": columns, "values": values}),
                )
                .unwrap();
            }
            Kind::Formula { cell, reference, .. } | Kind::FormulaError { cell, reference, .. } => {
                sheet::set(s, cell, &Cell::Formula(reference.clone())).unwrap()
            }
            Kind::Value { cell, expected } => sheet::set(s, cell, expected).unwrap(),
            Kind::Keys { combo } => keys.push(combo.clone()),
            Kind::Bold { range } => {
                s.execute("home.bold", json!({"range": range})).unwrap();
            }
            Kind::Filter => {
                s.execute("selection.set", json!({"range": task.focus})).unwrap();
                s.execute("data.filter", json!({})).unwrap();
            }
            Kind::FilterShows { col, first, value, .. } => {
                s.execute("selection.set", json!({"range": sheet::a1(*first, *col)})).unwrap();
                s.execute("data.filter", json!({})).unwrap();
                s.execute("data.filterBy", json!({"column": sheet::col_name(*col), "values": [value]})).unwrap();
            }
            Kind::Sorted { first, last, cols, key, descending } => {
                let range = format!("{}:{}", sheet::a1(*first, cols.0), sheet::a1(*last, cols.1));
                s.execute("data.sort", json!({"range": range, "header": false, "keys": [{"column": sheet::col_name(*key), "order": if *descending {"desc"} else {"asc"}}]}))
                    .unwrap();
            }
            Kind::Frozen { rows } => {
                s.execute("view.freezePanes", json!({"cell": format!("A{}", rows + 1)})).unwrap();
            }
            Kind::NumFmt { range, .. } => {
                s.execute("home.numberFormat", json!({"range": range, "code": "#,##0.00"})).unwrap();
            }
            Kind::Unique { col, first, last, .. } => {
                let range = format!("{}:{}", sheet::a1(*first, *col), sheet::a1(*last, col + 1));
                s.execute("data.removeDuplicates", json!({"range": range, "header": false})).unwrap();
            }
            Kind::Pairs { pairs } => {
                let refs: Vec<&str> = pairs.iter().map(|(_, f)| f.as_str()).collect();
                let values = sheet::probe(&task.setup, &refs).unwrap();
                for ((label, _), v) in pairs.iter().zip(values) {
                    sheet::set(s, &format!("Y{pair_row}"), &Cell::Text(label.clone())).unwrap();
                    sheet::set(s, &format!("Z{pair_row}"), &Cell::Num(v.as_f64().unwrap())).unwrap();
                    pair_row += 1;
                }
            }
        }
    }
    keys
}

fn assert_solvable(spec: &Spec) {
    let task = Task::generate(spec).unwrap_or_else(|e| panic!("{spec:?}: {e}"));
    if let Some(q) = &task.sql {
        let want = sql::run(&q.tables, &q.reference).unwrap_or_else(|e| panic!("{spec:?}: reference fails: {e}"));
        assert!(!want.rows.is_empty() || spec.skill == "sql.where" || spec.skill == "sql.like", "{spec:?} reference returns nothing");
        assert!(sql::evaluate(&task, &q.reference).unwrap().passed(), "{spec:?} reference query fails");
        assert!(!sql::evaluate(&task, "SELECT 1").unwrap().passed(), "{spec:?} passes with SELECT 1");
        // Hard-coding today's answer does not pass on other data.
        let literal = want
            .rows
            .iter()
            .map(|r| {
                format!(
                    "SELECT {}",
                    r.iter()
                        .map(|v| match v {
                            sql::SqlVal::Text(t) => format!("'{t}'"),
                            other => other.show(),
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join(" UNION ALL ");
        if !literal.is_empty() && !matches!(spec.skill.as_str(), "sql.distinct") {
            assert!(!sql::evaluate(&task, &literal).unwrap().passed(), "{spec:?} passes with literal rows");
        }
        return;
    }
    assert!(!task.checks.is_empty(), "{spec:?} has no checks");
    let mut s = task.session().unwrap();
    let before = evaluate(&task, &s, &[]).unwrap();
    assert!(!before.passed(), "{spec:?} passes untouched");
    let keys = solve(&task, &mut s);
    let after = evaluate(&task, &s, &keys).unwrap();
    let failed: Vec<_> = after.outcomes.iter().filter(|o| !o.passed).map(|o| format!("{}: {}", o.label, o.feedback)).collect();
    assert!(after.passed(), "{spec:?} reference solution fails: {failed:?}");
}

#[test]
fn every_skill_and_level_is_generated_and_its_reference_solution_passes() {
    for sk in SKILLS {
        for level in 1..=4 {
            for seed in [1u64, 7, 42] {
                assert_solvable(&Spec::new(sk.id, level, seed));
            }
        }
    }
}

#[test]
fn every_case_is_solvable() {
    for c in CASES {
        for level in 1..=4 {
            for seed in [3u64, 11] {
                assert_solvable(&Spec::new(c.id, level, seed));
            }
        }
    }
}

#[test]
fn every_skill_and_case_is_solvable_in_english() {
    gridcraft_tr::set_english_here(Some(true));
    for id in SKILLS.iter().map(|s| s.id).chain(CASES.iter().map(|c| c.id)) {
        for level in 1..=4 {
            for seed in [2u64, 13] {
                assert_solvable(&Spec::new(id, level, seed));
            }
        }
    }
    gridcraft_tr::set_english_here(None);
}

#[test]
fn vlookup_iferror_is_a_staged_ladder_and_hides_the_composed_answer_on_failure() {
    let ids: Vec<&str> = SKILLS.iter().filter(|s| s.unit == 5).map(|s| s.id).collect();
    let at = |id: &str| ids.iter().position(|x| *x == id).unwrap();
    assert!(at("lookup.vlookup") < at("lookup.error_observe"));
    assert!(at("lookup.error_observe") < at("lookup.iferror.wrap"));
    assert!(at("lookup.iferror.wrap") < at("lookup.iferror.combine"));
    assert!(at("lookup.iferror.combine") < at("lookup.xlookup"));

    let observe = Task::generate(&Spec::new("lookup.error_observe", 1, 17)).unwrap();
    assert!(observe.checks.iter().any(|c| matches!(&c.kind, Kind::FormulaError { error, .. } if error == "#N/A")));
    let visible = observe.example.as_deref().expect("worked VLOOKUP scaffold");
    // Both yellow cells get their own worked formula, each pointing at its own row.
    let lines: Vec<&str> = visible.lines().collect();
    assert_eq!(lines.len(), 2, "{visible}");
    assert_eq!(observe.answer_cells().len(), 2, "auto-check waits for both yellow cells");
    for (line, cell) in lines.iter().zip(observe.answer_cells()) {
        let (at, formula) = line.split_once(": ").unwrap();
        assert_eq!(at, cell);
        assert!(formula.starts_with("=ВПР(") && formula.ends_with(')'), "Russian Excel scaffold must be localized and closed: {formula}");
    }
    assert_ne!(lines[0].split_once(": ").unwrap().1, lines[1].split_once(": ").unwrap().1);
    gridcraft_tr::set_english_here(Some(true));
    let english = Task::generate(&Spec::new("lookup.error_observe", 1, 17)).unwrap();
    let visible = english.example.as_deref().expect("English VLOOKUP scaffold");
    assert!(visible.lines().all(|l| l.contains(": =VLOOKUP(") && l.ends_with(')')), "English scaffold must be localized and closed: {visible}");
    gridcraft_tr::set_english_here(None);

    let wrap = Task::generate(&Spec::new("lookup.iferror.wrap", 1, 17)).unwrap();
    assert!(wrap.setup.cells.iter().any(|(_, value, input)| !input && matches!(value, Cell::Formula(f) if f.contains("VLOOKUP"))));
    assert!(wrap.example.as_deref().is_some_and(|f| f.contains("IFERROR")));

    let combine = Task::generate(&Spec::new("lookup.iferror.combine", 2, 17)).unwrap();
    assert!(combine.example.is_none(), "the full nested answer is not revealed at scaffolded level 2");
    let result = evaluate(&combine, &combine.session().unwrap(), &[]).unwrap();
    assert!(!result.passed());
    assert!(result.outcomes.iter().filter(|o| !o.passed).all(|o| o.solution.is_none()), "first failure must not reveal the whole nested formula");
}

#[test]
fn lookup_iferror_ladder_requires_massed_practice_before_advancing() {
    let mut profile = Profile::default();
    let id = "lookup.iferror.combine";
    for attempt in 0..4 {
        let task = Task::generate(&Spec::new(id, 1, attempt)).unwrap();
        let change = profile.record(&task, true, 2.0, 0, 0, false, 1_800_000_000 + attempt);
        assert_eq!(profile.level(id), 1, "worked-example practice {attempt} must not rush ahead");
        assert!(change.level_up.is_none());
    }
    let task = Task::generate(&Spec::new(id, 1, 5)).unwrap();
    assert_eq!(profile.record(&task, true, 2.0, 0, 0, false, 1_800_000_005).level_up, Some(2));
    assert_eq!(profile.level(id), 2);

    for attempt in 0..4 {
        let task = Task::generate(&Spec::new(id, 2, 10 + attempt)).unwrap();
        let change = profile.record(&task, true, 10.0, 0, 0, false, 1_800_000_010 + attempt);
        assert_eq!(profile.level(id), 2, "tool-supported practice {attempt} must retain the stage");
        assert!(change.level_up.is_none());
    }
    let task = Task::generate(&Spec::new(id, 2, 15)).unwrap();
    profile.record(&task, true, 10.0, 0, 0, false, 1_800_000_015);
    assert_eq!(profile.level(id), 3);
}

#[test]
fn same_seed_same_exercise_and_layout_survives_new_data() {
    let a = Task::generate(&Spec::new("cond.sumif", 3, 5)).unwrap();
    let b = Task::generate(&Spec::new("cond.sumif", 3, 5)).unwrap();
    assert_eq!(a.setup.cells, b.setup.cells);
    let mut other = a.spec.clone();
    other.data += 1;
    let c = Task::generate(&other).unwrap();
    let cells = |t: &Task| t.setup.cells.iter().map(|(c, ..)| c.clone()).collect::<Vec<_>>();
    assert_eq!(cells(&a), cells(&c), "data seed must not move cells");
    assert_ne!(a.setup.cells, c.setup.cells);
}

#[test]
fn typed_result_and_broken_reference_are_rejected() {
    let task = Task::generate(&Spec::new("refs.abs", 2, 9)).unwrap();
    let mut s = task.session().unwrap();
    // Correct numbers typed by hand: right now, wrong on new data.
    for c in &task.checks {
        if let Kind::Formula { cell, reference, .. } = &c.kind {
            let v = sheet::probe(&task.setup, &[reference]).unwrap();
            sheet::set(&mut s, cell, &Cell::Num(v[0].as_f64().unwrap())).unwrap();
        }
    }
    let e = evaluate(&task, &s, &[]).unwrap();
    assert!(!e.passed());
    assert!(e.outcomes[0].feedback.contains("вручную"), "{}", e.outcomes[0].feedback);
    // Relative reference to the rate cell, filled down: first row right, the rest break.
    let mut s = task.session().unwrap();
    let cells: Vec<String> = task.answer_cells();
    let Kind::Formula { reference, .. } = &task.checks[0].kind else { panic!() };
    sheet::set(&mut s, &cells[0], &Cell::Formula(reference.replace('$', ""))).unwrap();
    s.execute("edit.fillDown", json!({"range": format!("{}:{}", cells[0], cells[cells.len() - 1])})).unwrap();
    let e = evaluate(&task, &s, &[]).unwrap();
    assert!(e.outcomes[0].passed || e.outcomes.len() == 1);
    assert!(!e.passed(), "unanchored rate must fail lower rows");
}

#[test]
fn equivalent_formulas_are_accepted() {
    let task = Task::generate(&Spec::new("start.mul", 1, 2)).unwrap();
    let Kind::Formula { cell, reference, .. } = &task.checks[0].kind else { panic!() };
    let (a, b) = reference.trim_start_matches('=').split_once('*').unwrap();
    let mut s = task.session().unwrap();
    sheet::set(&mut s, cell, &Cell::Formula(format!("={b}*{a}"))).unwrap();
    assert!(evaluate(&task, &s, &[]).unwrap().passed());
    let mut s = task.session().unwrap();
    sheet::set(&mut s, cell, &Cell::Formula(format!("=PRODUCT({a},{b})"))).unwrap();
    assert!(evaluate(&task, &s, &[]).unwrap().passed());
}

#[test]
fn editing_source_data_is_caught() {
    let task = Task::generate(&Spec::new("fn.sum", 2, 4)).unwrap();
    let mut s = task.session().unwrap();
    solve(&task, &mut s);
    let (cell, ..) = task.setup.cells.iter().find(|(_, _, i)| *i).unwrap();
    sheet::set(&mut s, cell, &Cell::Num(1.0)).unwrap();
    let e = evaluate(&task, &s, &[]).unwrap();
    assert!(!e.passed());
    assert!(e.outcomes[0].feedback.contains(&format!("исходную ячейку {cell}")), "{}", e.outcomes[0].feedback);
}

#[test]
fn russian_display_of_reference() {
    assert_eq!(sheet::ru("=SUMIF(A2:A9,H2,C2:C9)"), "=СУММЕСЛИ(A2:A9;H2;C2:C9)");
    assert_eq!(sheet::ru("=ROUND(B2*0.2,2)"), "=ОКРУГЛ(B2*0,2;2)");
    assert_eq!(sheet::num_text(1234567.5), "1 234 567,5");
    assert_eq!(sheet::date_text(sheet::date(2026, 3, 15)), "15.03.2026");
}

#[test]
fn staircase_climbs_on_clean_success_and_steps_down_after_two_misses() {
    let mut p = Profile::default();
    let now = 1_800_000_000;
    let climb = |p: &mut Profile, passed: bool, hints: u32| {
        let lvl = p.level("start.add").max(1);
        let t = Task::generate(&Spec::new("start.add", lvl, p.seed())).unwrap();
        p.record(&t, passed, 1000.0, hints, 0, false, now)
    };
    climb(&mut p, true, 0);
    let c = climb(&mut p, true, 0);
    assert_eq!(c.level_up, Some(2));
    climb(&mut p, true, 1);
    assert_eq!(p.level("start.add"), 2, "hinted success does not climb");
    climb(&mut p, false, 0);
    let c = climb(&mut p, false, 0);
    assert!(c.level_down);
    assert_eq!(p.level("start.add"), 1);
}

#[test]
fn units_and_skills_unlock_in_order_and_lessons_follow_the_frontier() {
    let mut p = Profile::default();
    assert!(p.unlocked("start.enter"));
    assert!(!p.unlocked("start.add"));
    assert!(p.unlocked("acc.accounts"), "accounting is open from the start");
    assert!(!p.unlocked("refs.fill"));
    assert_eq!(p.frontier(), Some("start.enter"));
    p.skills.entry("start.enter".into()).or_default().level = 2;
    assert!(p.unlocked("start.add"));
    let mut lesson = p.plan_lesson(1_800_000_000, None);
    assert_eq!(lesson.total, 8);
    let slot = lesson.queue.pop_front().unwrap();
    let spec = p.spec_for(&slot);
    assert_eq!(spec.skill, "start.enter", "frontier is still start.enter below level 3");
    for s in SKILLS.iter().filter(|s| s.unit == 0) {
        p.skills.entry(s.id.into()).or_default().level = 3;
    }
    assert!(p.unlocked("refs.fill"));
    assert_eq!(p.frontier(), Some("refs.fill"));
}

#[test]
fn streak_and_daily_goal() {
    let mut p = Profile::default();
    let day = 1_800_000_000u64;
    let t = Task::generate(&Spec::new("start.add", 1, 1)).unwrap();
    let c = p.record(&t, true, 5.0, 0, 0, false, day);
    assert!(c.streak_extended);
    assert_eq!(p.current_streak(day), 1);
    p.record(&t, true, 5.0, 0, 0, false, day + DAY);
    assert_eq!(p.current_streak(day + DAY), 2);
    assert_eq!(p.current_streak(day + 3 * DAY), 0);
    for _ in 0..5 {
        p.record(&t, true, 5.0, 0, 0, false, day + DAY);
    }
    assert!(p.today_xp(day + DAY) >= p.goal);
}

#[test]
fn interview_misses_send_skills_to_remediation() {
    let mut p = Profile::default();
    let t = Task::generate(&Spec::new("case.bank", 3, 2)).unwrap();
    let s = t.session().unwrap();
    let e = evaluate(&t, &s, &[]).unwrap();
    p.record_case(&t, &e, 600.0, Some(20), 1_800_000_000);
    assert!(p.remediation.contains(&"cond.countif".to_string()));
    assert_eq!(p.interviews.len(), 1);
    let lesson = p.plan_lesson(1_800_000_000, None);
    assert!(matches!(lesson.queue.front(), Some(Slot::Retry(_))));
}

#[test]
fn save_round_trip_resumes_the_exercise() {
    let mut p = Profile::default();
    let t = Task::generate(&Spec::new("lookup.vlookup", 2, 3)).unwrap();
    let mut s = t.session().unwrap();
    solve(&t, &mut s);
    p.record(&t, true, 20.0, 0, 0, false, 1_800_000_000);
    let save = Save {
        version: SAVE_VERSION,
        profile: p,
        current: Some(Current {
            task: t.clone(),
            workbook: (*s.doc().unwrap().wb).clone(),
            lesson: None,
            interview: None,
            elapsed: 3.0,
            hints: 0,
            keys: vec![],
            query: String::new(),
        }),
    };
    let back = Save::decode(&save.encode().unwrap()).unwrap();
    let cur = back.current.unwrap();
    assert!(evaluate(&cur.task, &cur.session(), &[]).unwrap().passed());
    assert_eq!(back.profile.log.len(), 1);
}

#[test]
fn speed_and_combo_earn_more_and_familiar_skills_move_faster() {
    let now = 1_800_000_000;
    let mut p = Profile::default();
    let t = Task::generate(&Spec::new("start.add", 2, 3)).unwrap();
    let slow = p.clone().record(&t, true, t.goal_seconds, 0, 0, false, now);
    let fast = p.clone().record(&t, true, t.goal_seconds * 0.3, 0, 4, false, now);
    assert_eq!(fast.speed, profile::Speed::Lightning);
    assert!(fast.xp >= slow.xp + 10 + 8, "{} vs {}", fast.xp, slow.xp);
    // Independent fast practice advances normally, not through the copying shortcut.
    let c = p.record(&t, true, t.goal_seconds * 0.3, 0, 0, false, now);
    assert!(!c.fast_track);
    assert_eq!(p.level("start.add"), 2);
    // A slow first try climbs normally.
    let t2 = Task::generate(&Spec::new("start.sub", 1, 3)).unwrap();
    p.record(&t2, true, t2.goal_seconds, 0, 0, false, now);
    assert_eq!(p.level("start.sub"), 1);
    // Test-out lesson: a clean pass at level 3 marks the skill as known.
    let lesson = p.plan_testout(1);
    assert!(lesson.total >= 3 && lesson.queue.iter().all(|s| matches!(s, Slot::TestOut(_))));
    let spec = p.spec_for(&Slot::TestOut("refs.fill".into()));
    assert_eq!(spec.level, 3);
    let t3 = Task::generate(&spec).unwrap();
    let c = p.record(&t3, true, t3.goal_seconds, 0, 0, true, now);
    assert!(c.fast_track);
    assert_eq!(p.level("refs.fill"), 3);
}

#[test]
fn freeze_wager_quests_and_badges() {
    let day = 1_800_000_000u64;
    let mut p = Profile::default();
    let t = Task::generate(&Spec::new("start.add", 2, 1)).unwrap();
    p.record(&t, true, 1000.0, 0, 0, false, day);
    assert!(p.achievements.is_empty() || !p.achievements.contains_key("streak_3"));
    p.gems = 300;
    p.buy_freeze().unwrap();
    assert_eq!(p.freezes, 1);
    p.place_wager(50, 2, day).unwrap();
    assert!(p.place_wager(50, 2, day).is_err());
    // Skip one day: the freeze keeps the streak.
    let c = p.record(&t, true, 1000.0, 0, 0, false, day + 2 * DAY);
    assert!(c.freeze_used);
    assert_eq!(p.current_streak(day + 2 * DAY), 2);
    assert_eq!(p.wager, None, "two days passed: the wager is won");
    assert_eq!(c.wager_won, Some(100));
    // Quests exist and progress.
    p.refresh_quests(day + 2 * DAY);
    assert_eq!(p.quests.len(), 3);
    assert_eq!(p.quests.first().unwrap().kind, profile::QuestKind::Xp);
    let c = p.finish_lesson(true, 10, day + 2 * DAY);
    assert!(c.achievements.contains(&"first_lesson".to_string()));
    assert!(p.achievements.contains_key("perfect_1") && p.achievements.contains_key("combo_10"));
    // Missing two days with no freeze breaks the streak and a new wager is lost.
    p.place_wager(20, 7, day + 2 * DAY).unwrap();
    let c = p.record(&t, true, 1000.0, 0, 0, false, day + 6 * DAY);
    assert!(c.wager_lost);
    assert_eq!(p.streak, 1);
}

#[test]
fn blitz_questions_have_one_right_answer() {
    for track in [quiz::Track::Excel, quiz::Track::Sql, quiz::Track::Mixed, quiz::Track::Supply] {
        for seed in 0..20u64 {
            let qs = quiz::blitz(seed, track, 15);
            assert_eq!(qs.len(), 15, "{track:?} {seed}");
            for q in &qs {
                assert!(q.options.len() >= 2 && q.correct < q.options.len(), "{q:?}");
                assert!(!q.explain.is_empty());
            }
        }
    }
    assert_eq!(quiz::blitz(5, quiz::Track::Mixed, 10), quiz::blitz(5, quiz::Track::Mixed, 10));
    assert_eq!(quiz::points(true, 0.0, 30), 20);
    assert_eq!(quiz::points(false, 0.0, 30), 0);
}

#[test]
fn sprint_plans_comparable_runs_and_keeps_records() {
    let now = 1_800_000_000;
    let mut p = Profile::default();
    p.skills.entry("start.add".into()).or_default().level = 2;
    let l = p.plan_sprint(None);
    assert_eq!(l.total, profile::SPRINT_LEN);
    assert_eq!(l.sprint.as_deref(), Some("mix"));
    let spec = p.spec_for(l.queue.front().unwrap());
    assert_eq!(spec.level, profile::SPRINT_LEVEL);
    let r = |s: f32| profile::SprintResult { at: now, kind: "mix".into(), tasks: 10, seconds: s, misses: 0, par: 600.0 };
    let (m, rec, _) = p.record_sprint(r(400.0), now);
    assert_eq!(m, profile::Medal::Bronze);
    assert!(rec);
    let (m, rec, c) = p.record_sprint(r(200.0), now);
    assert_eq!(m, profile::Medal::Gold);
    assert!(rec && c.achievements.contains(&"sprint_record".to_string()));
    let (_, rec, _) = p.record_sprint(r(300.0), now);
    assert!(!rec);
    assert_eq!(p.sprint_best("mix"), Some(200.0));
}

#[test]
fn english_mode_shows_no_russian() {
    gridcraft_tr::set_english_here(Some(true));
    let cyr = |s: &str| s.chars().any(|c| ('а'..='я').contains(&c.to_lowercase().next().unwrap_or(c)) || c == 'ё');
    let mut bad = std::collections::BTreeSet::new();
    // Report assembly is Russian-only for now (see KNOWN_LIMITATIONS.md).
    for id in SKILLS.iter().map(|s| s.id).filter(|id| !id.starts_with("asm.") && !id.starts_with("orion.")).chain(CASES.iter().map(|c| c.id)) {
        for level in 1..=4 {
            for seed in [2u64, 13, 5] {
                let t = Task::generate(&Spec::new(id, level, seed)).unwrap();
                let mut all = vec![t.title.clone(), t.prompt.clone()];
                all.extend(t.hints.clone());
                if let Some(b) = &t.brief {
                    all.push(format!("{b:?}"));
                }
                for (_, c, ..) in &t.setup.cells {
                    all.push(format!("{c:?}"));
                }
                if let Some(q) = &t.sql {
                    all.push(format!("{:?}", q.tables));
                }
                for s in all {
                    if cyr(&s) {
                        bad.insert(format!("{id}: {s}"));
                    }
                }
            }
        }
    }
    for seed in 0..40u64 {
        for track in [crate::quiz::Track::Excel, crate::quiz::Track::Mixed, crate::quiz::Track::Sql] {
            for q in crate::quiz::blitz(seed, track, 15) {
                for s in [q.topic.clone(), q.prompt.clone(), q.explain.clone(), q.options.join(" | "), format!("{:?}", q.table)] {
                    if cyr(&s) {
                        bad.insert(format!("quiz: {s}"));
                    }
                }
            }
        }
    }
    gridcraft_tr::set_english_here(None);
    assert!(bad.is_empty(), "Russian left in English mode: {bad:#?}");
}

#[test]
fn level_one_hints_never_repeat_the_formula_already_on_screen() {
    for sk in SKILLS {
        for seed in [3u64, 9] {
            let t = Task::generate(&Spec::new(sk.id, 1, seed)).unwrap();
            let Some(ex) = &t.example else { continue };
            for h in &t.hints {
                assert!(!h.contains(ex.as_str()), "{}: hint repeats the shown example: {h}", sk.id);
            }
        }
    }
    let t = Task::generate(&Spec::new("lookup.vlookup", 1, 4)).unwrap();
    assert!(t.hints.iter().any(|h| h.contains("что ищем") && h.contains("номер столбца")), "{:?}", t.hints);
}

#[test]
fn a_formula_pointing_at_the_neighbouring_row_is_named() {
    let task = Task::generate(&Spec::new("lookup.error_observe", 1, 17)).unwrap();
    let cells = task.answer_cells();
    let mut s = task.session().unwrap();
    // The learner copies the worked formula of the missing-code row into the found-code row.
    let Kind::FormulaError { reference, .. } = &task.checks[1].kind else { panic!() };
    sheet::set(&mut s, &cells[0], &Cell::Formula(reference.clone())).unwrap();
    sheet::set(&mut s, &cells[1], &Cell::Formula(reference.clone())).unwrap();
    let r = evaluate(&task, &s, &[]).unwrap();
    let first = &r.outcomes[0];
    assert!(!first.passed);
    assert!(first.feedback.contains("ссылка указывает не на ту ячейку"), "{}", first.feedback);
    assert!(r.outcomes[1].passed, "{}", r.outcomes[1].feedback);
}

#[test]
fn choosing_the_function_accepts_the_right_tool_and_rejects_its_look_alikes() {
    for id in ["choose.totals", "choose.lookup"] {
        for level in 1..=4u8 {
            for seed in 1..=12u64 {
                let task = Task::generate(&Spec::new(id, level, seed)).unwrap();
                assert!(task.example.is_none(), "{id}: naming the function is the exercise");
                assert_eq!(task.answer_cells().len(), if level == 4 { 3 } else { 1 });
                if level == 1 {
                    assert_eq!(task.prompt.matches("• ").count(), 3, "{}", task.prompt);
                }
                let cells = task.answer_cells();
                let refs: Vec<String> = task
                    .checks
                    .iter()
                    .filter_map(|c| if let Kind::Formula { reference, .. } = &c.kind { Some(reference.clone()) } else { None })
                    .collect();
                // The right formulas pass.
                let mut s = task.session().unwrap();
                for (cell, f) in cells.iter().zip(&refs) {
                    sheet::set(&mut s, cell, &Cell::Formula(f.clone())).unwrap();
                }
                assert!(evaluate(&task, &s, &[]).unwrap().passed(), "{id} L{level} s{seed}");
                // The look-alike tool (summing where the question counts, and so on) does not.
                let mut s = task.session().unwrap();
                let first = refs.first().unwrap();
                let (name, args) = first.trim_start_matches('=').trim_end_matches(')').split_once('(').unwrap();
                let args: Vec<&str> = args.split(',').collect();
                let amounts = |a: &str| a.replacen('A', "C", 2);
                let wrong = match name {
                    "SUM" => format!("=COUNT({})", args[0]),
                    "COUNT" => format!("=SUM({})", args[0]),
                    "MAX" | "AVERAGE" => format!("=MIN({})", args[0]),
                    "MIN" => format!("=MAX({})", args[0]),
                    "SUMIF" => format!("=COUNTIF({},{})", args[0], args[1]),
                    "COUNTIF" => format!("=SUMIF({},{},{})", args[0], args[1], amounts(args[0])),
                    "AVERAGEIF" => format!("=AVERAGE({})", args[2]),
                    "VLOOKUP" => format!("=MAX({})", args[1].replacen('B', "C", 1)),
                    other => panic!("{other}"),
                };
                sheet::set(&mut s, &cells[0], &Cell::Formula(wrong.clone())).unwrap();
                let r = evaluate(&task, &s, &[]).unwrap();
                assert!(!r.outcomes[0].passed, "{id} L{level} s{seed}: {wrong} accepted for {first}");
            }
        }
    }
}

#[test]
fn control_formulas_really_flag_the_broken_row_and_only_it() {
    for id in ["check.counttext", "check.reconcile", "check.catalog", "check.spaces"] {
        for level in 1..=4u8 {
            for seed in 1..=10u64 {
                let task = Task::generate(&Spec::new(id, level, seed)).unwrap();
                let broken = task
                    .checks
                    .iter()
                    .find_map(|c| match &c.kind {
                        Kind::Expected { expected: crate::check::Expectation::Number(n), .. } => Some(*n as u32),
                        _ => None,
                    })
                    .expect("row answer");
                let formulas: Vec<(String, String)> = task
                    .checks
                    .iter()
                    .filter_map(|c| if let Kind::Formula { cell, reference, .. } = &c.kind { Some((cell.clone(), reference.clone())) } else { None })
                    .collect();
                let values = sheet::probe(&task.setup, &formulas.iter().map(|(_, f)| f.as_str()).collect::<Vec<_>>()).unwrap();
                let nonzero = |v: &gridcraft_engine::core::Value| !matches!(v, gridcraft_engine::core::Value::Number(n) if n.abs() < 0.005);
                if formulas.len() == 1 {
                    assert!(nonzero(&values[0]), "{id} L{level} s{seed}: control is 0, the error is not caught");
                } else {
                    for ((cell, _), v) in formulas.iter().zip(&values) {
                        let row: u32 = cell.trim_start_matches(|c: char| c.is_ascii_alphabetic()).parse().unwrap();
                        // The catalog check flags with 0 (code not found); the others with not-0.
                        let flagged = if id == "check.catalog" { !nonzero(v) } else { nonzero(v) };
                        assert_eq!(flagged, row == broken, "{id} L{level} s{seed}: {cell} = {v:?}, broken row {broken}");
                    }
                }
                // A wrong row number does not pass.
                let mut s = task.session().unwrap();
                for (cell, f) in &formulas {
                    sheet::set(&mut s, cell, &Cell::Formula(f.clone())).unwrap();
                }
                let row_cell = task.answer_cells().into_iter().find(|c| !formulas.iter().any(|(f, _)| f == c)).unwrap();
                sheet::set(&mut s, &row_cell, &Cell::Num(f64::from(broken + 1))).unwrap();
                assert!(!evaluate(&task, &s, &[]).unwrap().passed(), "{id}: wrong row accepted");
                sheet::set(&mut s, &row_cell, &Cell::Num(f64::from(broken))).unwrap();
                assert!(evaluate(&task, &s, &[]).unwrap().passed(), "{id} L{level} s{seed}: right answer rejected");
            }
        }
    }
}

#[test]
fn pivot_ladder_takes_the_learners_clicks_and_names_the_count_slip() {
    for level in 1..=4u8 {
        for seed in 1..=6u64 {
            let task = Task::generate(&Spec::new("data.pivot", level, seed)).unwrap();
            let Kind::Pivot { rows, columns, values, .. } = task.checks[0].kind.clone() else { panic!() };
            let mut s = task.session().unwrap();
            // As the ribbon does it: a cell in the table, Insert → PivotTable with no parameters.
            s.execute("selection.set", json!({"range": task.focus})).unwrap();
            let r = s.execute("insert.pivotTable", json!({})).unwrap();
            let name = r["name"].as_str().map(str::to_string).unwrap_or_else(|| "PivotTable1".into());
            // Ticking fields in the pane: text → Rows, numbers → Values (Sum).
            for f in rows.iter().chain(&columns) {
                s.execute("pivot.addField", json!({"pivot": name, "field": f, "area": "rows"})).unwrap();
            }
            for f in &columns {
                s.execute("pivot.moveField", json!({"pivot": name, "field": f, "from": "rows", "to": "columns"})).unwrap();
            }
            let (amount, func) = values[0].clone();
            s.execute("pivot.addField", json!({"pivot": name, "field": amount, "area": "values"})).unwrap();
            let e = evaluate(&task, &s, &[]).unwrap();
            if func == gridcraft_engine::model::PivotFunc::Sum {
                assert!(e.passed(), "L{level} s{seed}: {:?}", e.outcomes.iter().map(|o| &o.feedback).collect::<Vec<_>>());
            } else {
                assert!(!e.passed());
                assert!(e.outcomes[0].feedback.contains("Итоги по: Количество"), "{}", e.outcomes[0].feedback);
                s.execute("pivot.valueSettings", json!({"pivot": name, "field": amount, "func": "count"})).unwrap();
                assert!(evaluate(&task, &s, &[]).unwrap().passed(), "L{level} s{seed}: count fix");
            }
        }
    }
}

#[test]
fn supply_tasks_need_max_zero_and_both_overdue_conditions() {
    for level in 1..=4u8 {
        for seed in 1..=8u64 {
            let task = Task::generate(&Spec::new("supply.need", level, seed)).unwrap();
            let refs: Vec<(String, String)> = task
                .checks
                .iter()
                .filter_map(|c| if let Kind::Formula { cell, reference, .. } = &c.kind { Some((cell.clone(), reference.clone())) } else { None })
                .collect();
            // The natural learner formula, order = MAX(0, need cell − stock − transit), passes.
            let mut s = task.session().unwrap();
            for (i, (cell, f)) in refs.iter().enumerate() {
                let f = if i % 2 == 1 {
                    let need = &refs[i - 1].0;
                    let row = need.trim_start_matches(|c: char| c.is_ascii_alphabetic());
                    let col = |k: u32| format!("{}{row}", sheet::col_name(sheet::address(need).unwrap().col - 4 + k));
                    format!("=MAX(0,{need}-{}-{})", col(2), col(3))
                } else {
                    f.clone()
                };
                sheet::set(&mut s, cell, &Cell::Formula(f)).unwrap();
            }
            assert!(evaluate(&task, &s, &[]).unwrap().passed(), "need L{level} s{seed}");
            // Without MAX the covered component goes negative and fails.
            for (cell, f) in refs.iter().skip(1).step_by(2) {
                sheet::set(&mut s, cell, &Cell::Formula(f.replacen("=MAX(0,", "=(", 1))).unwrap();
            }
            assert!(!evaluate(&task, &s, &[]).unwrap().passed(), "need L{level} s{seed}: negative order accepted");

            let task = Task::generate(&Spec::new("supply.overdue", level, seed)).unwrap();
            let mut s = task.session().unwrap();
            for c in &task.checks {
                if let Kind::Formula { cell, reference, .. } = &c.kind {
                    // Ignoring the status marks received orders too.
                    let f = reference.replacen("AND(", "AND(TRUE(),", 1);
                    let status_free = f.split(',').filter(|p| !p.contains("<>")).collect::<Vec<_>>().join(",");
                    sheet::set(&mut s, cell, &Cell::Formula(status_free)).unwrap();
                }
            }
            assert!(!evaluate(&task, &s, &[]).unwrap().passed(), "overdue L{level} s{seed}: status ignored but accepted");
        }
    }
}

#[test]
fn extra_practice_never_closes_a_finished_unit_or_the_next_skill() {
    let mut p = Profile::default();
    // A learner who finished units 0..=3 before the extras existed.
    for s in SKILLS.iter().filter(|s| s.unit <= 3 && !Profile::extra(s.id)) {
        p.skills.entry(s.id.into()).or_default().level = 2;
    }
    assert!(p.unlocked("cond.sumif"), "unit 4 must stay open despite the new unit-3 extra");
    assert!(p.unlocked("supply.need"), "an extra opens with the core skill before it");
    assert!(p.unlocked("fn.round"), "a core skill is gated by the previous core skill, not by an extra");
}

#[test]
fn the_autopilot_heads_for_supply_skills_through_their_prerequisites() {
    let mut p = Profile::default();
    assert_eq!(p.frontier(), Some("start.enter"), "a new learner starts at the beginning");
    // Units 0..=3 done: the course goes to СУММЕСЛИ before the rest of the path.
    for s in SKILLS.iter().filter(|s| s.unit <= 3 && !Profile::extra(s.id)) {
        p.skills.entry(s.id.into()).or_default().level = 3;
    }
    assert_eq!(p.frontier(), Some("cond.sumif"));
    for id in ["cond.sumif", "cond.countif", "supply.need"] {
        p.skills.entry(id.into()).or_default().level = 3;
    }
    // ВПР's unit is closed until unit 4 is done, so the next open goal leads on.
    let next = p.frontier().unwrap();
    assert!(next.starts_with("cond.") || next.starts_with("check.") || next.starts_with("data."), "{next}");
    // Pivot is locked behind sorting: the autopilot walks the unit in order, no skipping.
    for s in SKILLS.iter().filter(|s| s.unit == 4 && !Profile::extra(s.id)) {
        p.skills.entry(s.id.into()).or_default().level = 3;
    }
    p.skills.entry("check.reconcile".into()).or_default().level = 3;
    p.skills.entry("lookup.vlookup".into()).or_default().level = 3;
    // Hidden spaces are taught before live data starts hiding them.
    assert_eq!(p.frontier(), Some("text.trim"));
    p.skills.entry("text.trim".into()).or_default().level = 3;
    p.skills.entry("check.spaces".into()).or_default().level = 3;
    assert_eq!(p.frontier(), Some("data.sort"));
}

/// Tasks needed from level 3 to level 4 for a learner who errs at the given attempts.
fn tasks_to_level_four(misses: &[usize]) -> (usize, Vec<u8>) {
    let mut p = Profile::default();
    p.skills.entry("cond.sumif".into()).or_default().level = 3;
    let mut served = Vec::new();
    for i in 0..40 {
        let spec = p.spec_for(&Slot::Focus("cond.sumif".into()));
        served.push(spec.level);
        let task = Task::generate(&spec).unwrap();
        p.record(&task, !misses.contains(&i), task.goal_seconds * 0.8, 0, 0, false, 1_000_000 + i as u64 * 600);
        if p.level("cond.sumif") == 4 {
            return (i + 1, served);
        }
    }
    (usize::MAX, served)
}

#[test]
fn a_miss_costs_one_easier_task_not_a_long_detour() {
    assert_eq!(tasks_to_level_four(&[]).0, 3);
    let (n, served) = tasks_to_level_four(&[0]);
    assert_eq!(n, 5, "{served:?}");
    assert_eq!(served[1], 2, "one step easier right after the miss");
    assert_eq!(served[2], 3, "back on level once the easier task is solved");
    let (n, served) = tasks_to_level_four(&[0, 1]);
    assert_eq!(n, 7, "{served:?}");
    assert!(served.iter().all(|l| *l >= 2), "an independent learner never falls back to copying: {served:?}");
}

#[test]
fn sorting_one_selected_column_expands_to_the_table_as_excel_offers() {
    for seed in 1..=12u64 {
        let task = Task::generate(&Spec::new("data.sort", 1, seed)).unwrap();
        let Kind::Sorted { first, last, key, descending, .. } = task.checks[0].kind.clone() else { panic!() };
        let col = sheet::col_name(key);
        for cmd in [if descending { "data.sortDescending" } else { "data.sortAscending" }, "data.sort"] {
            let mut s = task.session().unwrap();
            s.execute("selection.set", json!({"range": format!("{col}{first}:{col}{last}")})).unwrap();
            let params =
                if cmd == "data.sort" { json!({"keys": [{"column": col, "order": if descending { "desc" } else { "asc" }}]}) } else { json!({}) };
            s.execute(cmd, params).unwrap();
            let e = evaluate(&task, &s, &[]).unwrap();
            assert!(e.passed(), "s{seed} {cmd}: {}", e.outcomes[0].feedback);
        }
    }
}

#[test]
fn one_spilling_formula_for_the_whole_column_counts_as_formulas() {
    for level in 1..=4u8 {
        let task = Task::generate(&Spec::new("text.trim", level, 5)).unwrap();
        let cells = task.answer_cells();
        let Kind::Formula { reference, .. } = task.checks[0].kind.clone() else { panic!() };
        // =TRIM(A2) in the first cell → =TRIM(A2:A7): one dynamic-array formula spills down.
        let first_src = reference.trim_start_matches("=TRIM(").trim_end_matches(')').to_string();
        let Kind::Formula { reference: last_ref, .. } = task.checks.last().unwrap().kind.clone() else { panic!() };
        let last_src = last_ref.trim_start_matches("=TRIM(").trim_end_matches(')').to_string();
        let mut s = task.session().unwrap();
        sheet::set(&mut s, &cells[0], &Cell::Formula(format!("=TRIM({first_src}:{last_src})"))).unwrap();
        let e = evaluate(&task, &s, &[]).unwrap();
        assert!(e.passed(), "L{level}: {:?}", e.outcomes.iter().filter(|o| !o.passed).map(|o| &o.feedback).collect::<Vec<_>>());
    }
}

#[test]
fn a_zero_in_quotes_is_named_as_text() {
    let task = Task::generate(&Spec::new("logic.iferror", 2, 3)).unwrap();
    let mut s = task.session().unwrap();
    for c in &task.checks {
        if let Kind::Formula { cell, reference, .. } = &c.kind {
            sheet::set(&mut s, cell, &Cell::Formula(reference.replacen(",0)", ",\"0\")", 1))).unwrap();
        }
    }
    let e = evaluate(&task, &s, &[]).unwrap();
    assert!(!e.passed());
    assert!(
        e.outcomes.iter().filter(|o| !o.passed).all(|o| o.feedback.contains("записано как текст")),
        "{:?}",
        e.outcomes.iter().map(|o| &o.feedback).collect::<Vec<_>>()
    );
}

#[test]
fn the_interview_progress_moves_with_the_learner_and_ends_at_one_hundred() {
    let mut p = Profile::default();
    let start = p.goal_plan(0);
    assert_eq!(start.percent, 0);
    assert!(start.passes_left > 0 && start.minutes_left > 0);
    p.skills.entry("start.enter".into()).or_default().level = 2;
    let after = p.goal_plan(0);
    assert!(after.percent >= start.percent && after.passes_left < start.passes_left);
    for s in SKILLS {
        p.skills.entry(s.id.into()).or_default().level = 3;
    }
    let done = p.goal_plan(0);
    assert_eq!((done.percent, done.passes_left), (100, 0));
}

#[test]
fn the_preparation_sprint_serves_only_the_goal_then_confidence_practice() {
    let now = 100_000_000;
    let mut p = Profile { sprint_until: now + 86_400, ..Profile::default() };
    // Units 0..=4 done: the goal route continues; old mastered skills are not mixed in.
    for s in SKILLS.iter().filter(|s| s.unit <= 4) {
        let e = p.skills.entry(s.id.into()).or_default();
        e.level = 3;
        e.card.review(fsrs::Grade::Good, now - 86_400 * 30);
    }
    let lesson = p.plan_lesson(now, None);
    let reviews = lesson.queue.iter().filter(|s| matches!(s, Slot::Review(_))).count();
    assert!(reviews <= 1 && !lesson.queue.iter().any(|s| matches!(s, Slot::Mix(_))), "{:?}", lesson.queue);
    // The lookup ladder uses ordinary thresholds in the sprint: two clean passes leave level 1.
    let task = Task::generate(&Spec::new("lookup.error_observe", 1, 3)).unwrap();
    p.skills.remove("lookup.error_observe");
    for i in 0..2 {
        p.record(&task, true, 30.0, 0, 0, false, now + i);
    }
    assert_eq!(p.level("lookup.error_observe"), 2);
    // Goal reached: every task is a goal skill, for confidence.
    for s in SKILLS {
        p.skills.entry(s.id.into()).or_default().level = 3;
    }
    assert!(p.goal_reached());
    let lesson = p.plan_lesson(now, None);
    assert!(lesson.queue.iter().all(|s| matches!(s, Slot::Focus(_))), "{:?}", lesson.queue);
    // After the deadline the ordinary course returns.
    let lesson = p.plan_lesson(now + 2 * 86_400, None);
    assert!(lesson.queue.iter().any(|s| !matches!(s, Slot::Focus(_))));
}

#[test]
fn fixing_the_broken_code_names_the_cell_and_says_to_find_not_fix() {
    let task = Task::generate(&Spec::new("check.catalog", 2, 4)).unwrap();
    assert!(task.prompt.contains("только найти строку"));
    let broken = task
        .checks
        .iter()
        .find_map(|c| match &c.kind {
            Kind::Expected { expected: crate::check::Expectation::Number(n), .. } => Some(*n as u32),
            _ => None,
        })
        .unwrap();
    let mut s = task.session().unwrap();
    sheet::set(&mut s, &format!("A{broken}"), &Cell::Text("A-1".into())).unwrap();
    let e = evaluate(&task, &s, &[]).unwrap();
    assert!(
        e.outcomes.iter().any(|o| o.feedback.contains(&format!("исходную ячейку A{broken}"))),
        "{:?}",
        e.outcomes.iter().map(|o| &o.feedback).collect::<Vec<_>>()
    );
}



