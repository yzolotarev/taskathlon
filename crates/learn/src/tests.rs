#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::check::Kind;
use crate::content::{CASES, SKILLS};
use crate::profile::DAY;
use crate::sheet::{self, Cell};
use crate::*;
use gridcraft_engine::Session;
use serde_json::json;

/// Plays the reference solution for every check of a task. Returns the keys "pressed".
fn solve(task: &Task, s: &mut Session) -> Vec<String> {
    let mut keys = Vec::new();
    let mut pair_row = 1;
    for c in &task.checks {
        match &c.kind {
            Kind::Formula { cell, reference, .. } => sheet::set(s, cell, &Cell::Formula(reference.clone())).unwrap(),
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
            Kind::Amount { cell, expected, .. } => sheet::set(s, cell, &Cell::Num(*expected)).unwrap(),
            Kind::Findings { items } => {
                for (no, diff) in items {
                    sheet::set(s, &format!("Y{pair_row}"), &Cell::Text(format!("№ {no}"))).unwrap();
                    sheet::set(s, &format!("Z{pair_row}"), &Cell::Num(-diff)).unwrap();
                    pair_row += 1;
                }
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
    assert!(e.outcomes[0].feedback.contains("Исходные"));
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
    let t = Task::generate(&Spec::new("start.add", 1, 3)).unwrap();
    let slow = p.clone().record(&t, true, t.goal_seconds, 0, 0, false, now);
    let fast = p.clone().record(&t, true, t.goal_seconds * 0.3, 0, 4, false, now);
    assert_eq!(fast.speed, profile::Speed::Lightning);
    assert!(fast.xp >= slow.xp + 10 + 8, "{} vs {}", fast.xp, slow.xp);
    // First try, fast and clean: straight to level 3.
    let c = p.record(&t, true, t.goal_seconds * 0.3, 0, 0, false, now);
    assert!(c.fast_track);
    assert_eq!(p.level("start.add"), 3);
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
    for track in [quiz::Track::Excel, quiz::Track::Sql, quiz::Track::Mixed] {
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
    for id in SKILLS.iter().map(|s| s.id).chain(CASES.iter().map(|c| c.id)) {
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
