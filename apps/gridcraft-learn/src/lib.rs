//! Taskathlon: open the app, press one button, practise in a real spreadsheet.
#![forbid(unsafe_code)]
#![cfg(not(target_arch = "wasm32"))]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented, clippy::unreachable)]

use gridcraft_tr::{t as tr, ts};
mod ai;
pub mod events;
pub mod ledger_ui;
pub mod remind;
mod screens;
pub mod sound;
mod supply;
pub mod view;

use gridcraft_learn_core::{
    Current, Evaluation, Interview, LearnError, Lesson, Mode, Profile, SAVE_VERSION, Save, Slot, Spec, Task,
    content::{self, CASES},
    evaluate,
    profile::{Change, Medal, SPRINT_PENALTY, Speed, SprintResult, achievement},
    quiz::{self, BlitzResult, Question, Track},
    sql::{self, ResultSet},
};
use gridcraft_ui_egui::{Services, SheetApp};
use sound::Cue;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[expect(clippy::disallowed_methods, reason = "Clock belongs to the native-only desktop shell")]
pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[expect(clippy::disallowed_methods, reason = "Clock belongs to the native-only desktop shell")]
pub(crate) fn instant() -> Instant {
    Instant::now()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Home,
    Supply,
    Lesson,
    Done,
    Case,
    Report,
    Stats,
    Blitz,
    BlitzReport,
    Ledger,
}

/// Stable label of a screen for the local action log.
fn screen_name(s: Screen) -> &'static str {
    match s {
        Screen::Home => "home",
        Screen::Supply => "supply",
        Screen::Lesson => "lesson",
        Screen::Done => "done",
        Screen::Case => "case",
        Screen::Report => "report",
        Screen::Stats => "stats",
        Screen::Blitz => "blitz",
        Screen::BlitzReport => "blitz_report",
        Screen::Ledger => "ledger",
    }
}

/// "ctrl+shift+alt" style label for the input modality of a key/pointer event.
fn modifiers_label(m: egui::Modifiers) -> String {
    let mut parts = Vec::new();
    if m.command || m.ctrl {
        parts.push("ctrl");
    }
    if m.shift {
        parts.push("shift");
    }
    if m.alt {
        parts.push("alt");
    }
    parts.join("+")
}

/// Fields for a `task.start` event: which exercise, where it came from and its seed.
fn task_start_fields(task: &Task, lesson: Option<&Lesson>, interview: Option<&Interview>, slot: Option<&Slot>) -> serde_json::Value {
    let stage = task.spec.skill.strip_prefix(supply::PREFIX).and_then(|s| s.parse::<usize>().ok());
    let source = if stage.is_some() {
        "supply"
    } else if interview.is_some() {
        "interview"
    } else if lesson.is_some_and(|l| l.sprint.is_some()) {
        "sprint"
    } else if slot.is_some_and(|s| matches!(s, Slot::TestOut(_))) {
        "testout"
    } else if lesson.is_some() {
        "lesson"
    } else if task.mode != Mode::Drill {
        "case"
    } else {
        "load"
    };
    serde_json::json!({
        "source": source,
        "skill": task.spec.skill,
        "stage": stage,
        "level": task.spec.level,
        "seed": task.spec.layout,
        "layout": task.spec.layout,
        "data": task.spec.data,
        "mode": format!("{:?}", task.mode),
    })
}

/// A snapshot of the supply route, used to notice stage advances and pause/resume.
#[derive(Clone, PartialEq)]
struct SupplySnap {
    stage: Option<usize>,
    paused: bool,
    complete: bool,
    feedback: Option<String>,
}

/// A short message that floats in and fades (badges, quests, gems).
pub struct Toast {
    pub icon: String,
    pub text: String,
    pub born: f64,
}

/// A big centred word for a moment ("⚡ МОЛНИЯ!", "КОМБО ×5").
pub struct Pop {
    pub text: String,
    pub color: egui::Color32,
    pub born: f64,
}

pub struct Blitz {
    pub track: Track,
    pub questions: Vec<Question>,
    pub index: usize,
    pub started: Instant,
    pub began: Instant,
    /// (chosen option, seconds used, points) per answered question.
    pub answers: Vec<(Option<usize>, f32, u32)>,
    pub combo: u32,
}

impl Blitz {
    pub fn current(&self) -> Option<&Question> {
        self.questions.get(self.index)
    }
    pub fn used(&self) -> f32 {
        self.started.elapsed().as_secs_f32()
    }
    pub fn answered(&self) -> bool {
        self.answers.len() > self.index
    }
    pub fn points(&self) -> u32 {
        self.answers.iter().map(|a| a.2).sum()
    }
    pub fn correct(&self) -> u32 {
        self.answers.iter().zip(&self.questions).filter(|(a, q)| a.0 == Some(q.correct)).count() as u32
    }
}

/// 2026-10-20 10:00 in Almaty (UTC+5), unix seconds.
pub const INTERVIEW_AT: u64 = 1_792_472_400;

pub struct Active {
    pub task: Task,
    pub lesson: Option<Lesson>,
    pub interview: Option<Interview>,
    pub slot: Option<Slot>,
    started: Instant,
    before: f32,
    pub hints: u32,
    /// The first evaluation of a drill is what counts for the staircase.
    pub recorded: bool,
    /// SQL drills: the query being written and the last run.
    pub query: String,
    pub sql_result: Option<Result<ResultSet, String>>,
    /// Progress of the skill just before its first grade, so a disputed miss can be undone.
    pub before_grade: Option<BeforeGrade>,
}

/// What the first grade of an exercise may change, kept to undo it on «Мой ответ верный».
#[derive(Clone)]
pub struct BeforeGrade {
    pub skill: String,
    pub progress: Option<gridcraft_learn_core::profile::SkillProgress>,
    pub live: Option<gridcraft_learn_core::profile::LiveProgress>,
}

impl Active {
    pub fn seconds(&self) -> f32 {
        self.before + self.started.elapsed().as_secs_f32()
    }
}

pub struct Feedback {
    pub passed: bool,
    pub eval: Evaluation,
    pub xp: u32,
    pub level_up: Option<u8>,
    pub level_down: bool,
    pub goal_reached: bool,
    /// Passed only after a correction: no XP, but the exercise is done.
    pub corrected: bool,
    pub change: Change,
    /// An extra line for the banner (the live-data walkthrough after two misses).
    pub note: Option<String>,
}

pub struct SprintSummary {
    pub kind: String,
    pub seconds: f32,
    pub misses: u32,
    pub medal: Medal,
    pub record: bool,
    pub previous: Option<f32>,
    pub par: f32,
}

pub struct Summary {
    pub sprint: Option<SprintSummary>,
    pub perfect: bool,
    pub gems: u32,
    pub test_out: bool,
    pub xp: u32,
    pub accuracy: f32,
    pub seconds: f32,
    pub best_combo: u32,
    pub level_ups: Vec<String>,
    pub streak: u32,
    pub streak_extended: bool,
}

pub struct Report {
    pub task: Task,
    pub eval: Evaluation,
    pub seconds: f32,
    pub interview: bool,
    pub xp: u32,
    pub timed_out: bool,
}

pub struct LearningApp {
    pub supply: Option<supply::Route>,
    pub sheet: SheetApp,
    pub profile: Profile,
    /// Local-only action log (see [`events`]); inert without a persistent path or when off.
    pub events: events::EventLog,
    /// Identifier shared by every event of this run.
    pub session_id: u64,
    pub screen: Screen,
    pub active: Option<Active>,
    pub feedback: Option<Feedback>,
    pub summary: Option<Summary>,
    /// The accounting program, loaded on first open.
    pub ledger: Option<ledger_ui::LedgerView>,
    /// When the summary screen opened (for its count-up animation).
    pub done_since: Option<f64>,
    pub report: Option<Report>,
    pub notice: Option<String>,
    /// Shortcuts pressed during the current exercise ("Ctrl+D", "F4", …).
    pub keys: Vec<String>,
    path: Option<PathBuf>,
    last_save: Instant,
    graded_revision: Option<u64>,
    /// A drill of this skill just passed: its live-data step opens below it in the same book.
    live_after: Option<String>,
    confetti: Vec<view::Particle>,
    pending_confetti: bool,
    enter: bool,
    ctrl_enter: bool,
    streak_extended: bool,
    show_settings: bool,
    show_hints: u32,
    ai_config: Option<ai::Config>,
    ai_consent: bool,
    ai_result: Option<String>,
    ai_pending: Option<(u64, std::sync::mpsc::Receiver<Result<String, String>>)>,
    ai_last: Option<Instant>,
    ai_requests: u32,
    clock: Instant,
    pub toasts: Vec<Toast>,
    pub pop: Option<Pop>,
    pub blitz: Option<Blitz>,
    pub blitz_result: Option<(Blitz, Change)>,
    /// Daily motivation shown on the home screen.
    pub greeting: Option<String>,
    remind_status: Option<String>,
    blitz_key: Option<usize>,
    sprint_clock: Option<Instant>,
    /// Text typed since the last commit, for the "what was typed where" log.
    typed: String,
    /// Whether a pointer button is currently held (drags are logged, moves are not).
    pointer_down: bool,
    /// Last screen seen by the observer, so transitions are logged once.
    logged_screen: Option<Screen>,
    /// Last supply snapshot, so stage/pause changes become log events.
    supply_snap: Option<SupplySnap>,
}

impl LearningApp {
    pub fn new(path: Option<PathBuf>) -> Result<Self, LearnError> {
        let mut notice = None;
        let save = match path.as_ref() {
            Some(p) if p.exists() => {
                let bytes = std::fs::read(p).map_err(|e| LearnError::Invalid(tr(&format!("Не удалось прочитать {}: {e}", p.display()))))?;
                match Save::decode(&bytes) {
                    Ok(s) => Some(s),
                    Err(_) => {
                        // An older or damaged file is kept aside, never overwritten.
                        let backup = p.with_extension("old.json");
                        std::fs::rename(p, &backup)
                            .map_err(|e| LearnError::Invalid(tr(&format!("Не удалось сохранить копию старого прогресса: {e}"))))?;
                        notice = Some(tr(&format!("Старый файл прогресса сохранён как {}. Начинаем новую версию тренажёра.", backup.display())));
                        None
                    }
                }
            }
            _ => None,
        };
        let (mut profile, current) = save.map(|s| (s.profile, s.current)).unwrap_or_default();
        // The learner's job interview (Орион Система, 20.10 10:00 Алматы): prepare toward it.
        // Only for a real progress file, never in tests or ephemeral runs.
        if path.is_some() && profile.sprint_until == 0 && now() < INTERVIEW_AT {
            profile.sprint_until = INTERVIEW_AT;
        }
        let mut sheet = SheetApp::new(gridcraft_engine::Session::new(), Services::default());
        sheet.ui.dark = profile.dark;
        gridcraft_tr::set_english(profile.english);
        sheet.ui.language = if profile.english { gridcraft_ui_egui::i18n::Language::En } else { gridcraft_ui_egui::i18n::Language::Ru };
        sheet.ui.ribbon_collapsed = true;
        sheet.ui.animations = false;
        sheet.ui.interface_scale = gridcraft_ui_egui::clamp_interface_scale(f64::from(profile.interface_scale)).unwrap_or(100);
        let events = events::EventLog::new(path.as_deref(), profile.logging);
        let session_id = events.session_id();
        let mut app = Self {
            supply: None,
            sheet,
            profile,
            events,
            session_id,
            screen: Screen::Home,
            active: None,
            feedback: None,
            summary: None,
            ledger: None,
            done_since: None,
            report: None,
            notice,
            keys: Vec::new(),
            path,
            last_save: instant(),
            graded_revision: None,
            live_after: None,
            confetti: Vec::new(),
            pending_confetti: false,
            enter: false,
            ctrl_enter: false,
            streak_extended: false,
            show_settings: false,
            show_hints: 0,
            ai_config: ai::Config::from_env(),
            ai_consent: false,
            ai_result: None,
            ai_pending: None,
            ai_last: None,
            ai_requests: 0,
            clock: instant(),
            toasts: Vec::new(),
            pop: None,
            blitz: None,
            blitz_result: None,
            greeting: None,
            remind_status: None,
            blitz_key: None,
            sprint_clock: None,
            typed: String::new(),
            pointer_down: false,
            logged_screen: None,
            supply_snap: None,
        };
        app.greeting = Some(remind::greeting(&app.profile, now()));
        if let Some(c) = current {
            // One course position: an older progress that predates `cursor` migrates from `current`.
            if app.profile.cursor.is_none() {
                app.profile.cursor = Some(c.task.spec.skill.clone());
            }
            let supply_stage = c.task.spec.skill.strip_prefix(supply::PREFIX).and_then(|s| s.parse::<usize>().ok()).filter(|s| *s < 12);
            if supply_stage.is_some() {
                if app.profile.supply_stage.is_none() {
                    app.profile.supply_stage = supply_stage.map(|s| s as u8);
                }
                app.supply = Some(supply::Route {
                    quiet: c.query.contains("quiet"),
                    paused: c.query.contains("paused"),
                    elapsed: c.elapsed,
                    started: instant(),
                    feedback: if c.query.contains("complete") {
                        Some("Заказы отправлены. Формулы работают и на изменённых данных.".into())
                    } else {
                        None
                    },
                    complete: c.query.contains("complete"),
                });
            }
            app.sheet.session = c.session();
            if supply_stage.is_some() {
                app.sheet.session.execute("selection.set", serde_json::json!({"range": c.task.focus}))?;
            }
            app.keys = c.keys.clone();
            app.screen = if c.task.mode == Mode::Drill { Screen::Lesson } else { Screen::Case };
            if supply_stage.is_some() {
                app.screen = Screen::Supply;
            }
            app.show_hints = c.hints;
            app.active = Some(Active {
                task: c.task,
                lesson: c.lesson,
                interview: c.interview,
                slot: None,
                started: instant(),
                before: c.elapsed,
                hints: c.hints,
                recorded: false,
                query: c.query,
                sql_result: None,
                before_grade: None,
            });
            app.graded_revision = if supply_stage.is_some() { None } else { app.revision() };
            app.apply_ribbon();
        }
        app.apply_zoom();
        Ok(app)
    }

    fn revision(&self) -> Option<u64> {
        self.sheet.session.doc().ok().map(|d| d.revision)
    }

    /// The zoom of the active sheet, persisted with the profile.
    fn current_zoom(&self) -> u16 {
        self.sheet.session.doc().ok().and_then(|d| d.wb.sheets.get(d.wb.active_sheet).map(|s| s.zoom)).unwrap_or(100)
    }

    /// Re-applies the learner's saved zoom to the fresh workbook a `load` rebuilds.
    fn apply_zoom(&mut self) {
        let z = self.profile.sheet_zoom.clamp(10, 400);
        let _ = self.sheet.session.execute("view.zoom", serde_json::json!({"percent": z}));
    }

    /// The A1 text of the current selection, the "where" of typed text.
    fn current_cell(&self) -> String {
        self.sheet.session.doc().map(|d| d.selection.a1()).unwrap_or_default()
    }

    /// Local-only event with the current screen attached.
    fn log_event(&mut self, kind: &str, fields: serde_json::Value) {
        self.events.set_screen(screen_name(self.screen));
        self.events.log(kind, fields);
    }

    /// The mistakes journal: every wrong answer with what was typed and what the grader said,
    /// and every later correction, so the course can be adapted to the learner's real errors.
    fn log_mistake(&self, fb: &Feedback, first: bool, hints: u32, seconds: f32) {
        if fb.passed && first {
            return;
        }
        let result = if fb.passed {
            "fixed"
        } else if first {
            "wrong"
        } else {
            "wrong_again"
        };
        self.log_mistake_as(fb, result, hints, seconds);
    }

    fn log_mistake_as(&self, fb: &Feedback, result: &str, hints: u32, seconds: f32) {
        let Some(a) = self.active.as_ref() else { return };
        let task = &a.task;
        let wb = self.sheet.session.doc().ok().map(|d| d.wb.clone());
        let mut cells = task.answer_cells();
        // A self-placed live answer has no fixed cell: find it by its «Ответ» label.
        if let (Some(wb), Some(gridcraft_learn_core::check::Kind::Live { cell: None, top, .. })) = (wb.as_ref(), task.checks.first().map(|c| &c.kind))
            && let Some(at) = gridcraft_learn_core::check::live_answer_cell(task, wb, *top)
        {
            cells.push(at.a1());
        }
        let typed: Vec<serde_json::Value> = cells
            .iter()
            .map(|cell| {
                let input = wb
                    .as_ref()
                    .and_then(|wb| {
                        let at = gridcraft_engine::core::CellRef::parse(cell)?;
                        let sh = wb.sheet(wb.sheet_index(&gridcraft_learn_core::check::home_sheet()).unwrap_or(0))?;
                        sh.cell(at).map(|c| c.input_text())
                    })
                    .unwrap_or_default();
                serde_json::json!({"cell": cell, "typed": input})
            })
            .collect();
        let wrong: Vec<serde_json::Value> = fb
            .eval
            .outcomes
            .iter()
            .filter(|o| !o.passed)
            .map(|o| serde_json::json!({"check": o.label, "said": o.feedback, "right": o.solution}))
            .collect();
        let prompt: String = task.prompt.chars().take(400).collect();
        self.events.mistake(serde_json::json!({
            "result": result,
            "skill": task.spec.skill,
            "name": task.skill_name(),
            "level": task.spec.level,
            "title": task.title,
            "prompt": prompt,
            "typed": typed,
            "wrong": wrong,
            "hints": hints,
            "seconds": seconds.round(),
            "keys": self.keys,
            "query": a.query,
        }));
    }

    /// Failing cells of the active task, as graded by the engine.
    fn failing_cells(&self) -> Vec<String> {
        self.active
            .as_ref()
            .map(|a| {
                evaluate(&a.task, &self.sheet.session, &self.keys)
                    .map(|e| e.outcomes.into_iter().filter(|o| !o.passed).map(|o| o.label).collect())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }

    /// Deterministic formula-error classes for the answer cells of the active task, for later
    /// remediation and offline analysis. Local-only data collection; never blocks practice.
    fn formula_issues(&self) -> serde_json::Value {
        let Some(task) = self.active.as_ref().map(|a| &a.task) else { return serde_json::Value::Array(Vec::new()) };
        let cells: Vec<String> = task
            .checks
            .iter()
            .filter_map(|c| match &c.kind {
                gridcraft_learn_core::check::Kind::Formula { cell, .. } | gridcraft_learn_core::check::Kind::FormulaError { cell, .. } => {
                    Some(cell.clone())
                }
                _ => None,
            })
            .collect();
        if cells.is_empty() {
            return serde_json::Value::Array(Vec::new());
        }
        let Ok(doc) = self.sheet.session.doc() else { return serde_json::Value::Array(Vec::new()) };
        let Some(sh) = doc.wb.sheets.get(doc.wb.active_sheet) else { return serde_json::Value::Array(Vec::new()) };
        use gridcraft_learn_core::formula_diagnosis::{FormulaIssue, diagnose, practice_hint, skill_for};
        let mut out = Vec::new();
        for cell in cells {
            let Some(at) = gridcraft_engine::core::CellRef::parse(&cell) else { continue };
            let Some(c) = sh.cell(at) else { continue };
            let raw = match &c.formula {
                Some(f) => format!("={}", f.text),
                None => match &c.value {
                    gridcraft_engine::core::Value::Error(e) => e.as_str().to_string(),
                    _ => continue,
                },
            };
            let issue = diagnose(&raw);
            if issue != FormulaIssue::Other {
                out.push(serde_json::json!({
                    "cell": cell,
                    "issue": format!("{issue:?}"),
                    "hint": practice_hint(&issue),
                    "skill": skill_for(&issue),
                }));
            }
        }
        serde_json::Value::Array(out)
    }

    /// Logs a screen transition once per change.
    fn observe_screen(&mut self) {
        if self.logged_screen != Some(self.screen) {
            let from = self.logged_screen.map(screen_name).unwrap_or("none");
            let to = screen_name(self.screen);
            self.log_event("screen", serde_json::json!({"from": from, "to": to}));
            self.logged_screen = Some(self.screen);
        }
    }

    /// The supply route lives in its own module; its progress is observed here so stage
    /// advances and pause/resume still reach the action log.
    fn observe_supply(&mut self) {
        let snap = SupplySnap {
            stage: self.supply_stage(),
            paused: self.supply.as_ref().is_some_and(|r| r.paused),
            complete: self.supply.as_ref().is_some_and(|r| r.complete),
            feedback: self.supply.as_ref().and_then(|r| r.feedback.clone()),
        };
        if let Some(prev) = self.supply_snap.clone() {
            if snap.paused != prev.paused {
                self.log_event(if snap.paused { "supply_pause" } else { "supply_resume" }, serde_json::json!({"stage": snap.stage}));
            }
            if self.screen == Screen::Supply && snap.stage != prev.stage && snap.stage > prev.stage {
                let seconds = self.supply.as_ref().map(|r| r.seconds()).unwrap_or(0.0);
                let hints = self.active.as_ref().map(|a| a.hints).unwrap_or(0);
                self.log_event(
                    "check_supply",
                    serde_json::json!({"passed": true, "stage": snap.stage, "seconds": seconds, "hints": hints, "failing": []}),
                );
            }
            if self.screen == Screen::Supply
                && snap.feedback != prev.feedback
                && let Some(text) = snap.feedback.clone()
                && !text.starts_with('✓')
                && !text.starts_with("Заказы")
                && !text.starts_with("Верно")
            {
                let failing = self.failing_cells();
                self.log_event("check_supply", serde_json::json!({"passed": false, "stage": snap.stage, "failing": failing, "feedback": text}));
            }
        }
        self.supply_snap = Some(snap);
    }

    pub fn save(&mut self) -> Result<(), LearnError> {
        self.last_save = instant();
        self.profile.interface_scale = self.sheet.ui.interface_scale;
        self.profile.sheet_zoom = self.current_zoom();
        // The active exercise (including a supply stage advance) is always the course position.
        if let Some(a) = &self.active {
            self.profile.set_cursor(&a.task.spec.skill);
            // The route keeps its own bookmark so the skill line never loses it.
            if let Some(stage) = a.task.spec.skill.strip_prefix(supply::PREFIX).and_then(|s| s.parse::<usize>().ok()).filter(|s| *s < supply::STAGES)
            {
                self.profile.supply_stage = Some(stage as u8);
            }
        }
        let active = self.active.is_some();
        self.log_event("save", serde_json::json!({"active": active}));
        let Some(path) = self.path.clone() else { return Ok(()) };
        let current = match &self.active {
            Some(a) => Some(Current {
                task: a.task.clone(),
                workbook: (*self.sheet.session.doc()?.wb).clone(),
                lesson: a.lesson.clone(),
                interview: a.interview.clone(),
                elapsed: self.supply.as_ref().filter(|_| self.supply_stage().is_some()).map_or_else(|| a.seconds(), |r| r.seconds()),
                hints: a.hints,
                keys: self.keys.clone(),
                query: if self.supply_stage().is_some() {
                    self.supply
                        .as_ref()
                        .map(|r| {
                            format!(
                                "{} {} {}",
                                if r.paused { "paused" } else { "" },
                                if r.complete { "complete" } else { "" },
                                if r.quiet { "quiet" } else { "" }
                            )
                        })
                        .unwrap_or_default()
                } else {
                    a.query.clone()
                },
            }),
            None => None,
        };
        self.profile.dark = self.sheet.ui.dark;
        let save = Save { version: SAVE_VERSION, profile: self.profile.clone(), current };
        atomic_write(&path, &save.encode()?)
    }

    fn handle(&mut self, r: Result<(), LearnError>) {
        if let Err(e) = r {
            self.notice = Some(e.to_string());
        }
    }

    fn cue(&self, cue: Cue) {
        if self.profile.sound && self.path.is_some() {
            sound::play(cue);
        }
    }

    /// Data-tool lessons and cases need the ribbon; formula drills stay distraction-free.
    fn apply_ribbon(&mut self) {
        let need = self.active.as_ref().is_some_and(|a| a.task.spec.skill.starts_with("data.") || a.task.mode != Mode::Drill);
        self.sheet.ui.ribbon_collapsed = !need;
    }

    fn load(&mut self, task: Task, lesson: Option<Lesson>, interview: Option<Interview>, slot: Option<Slot>) -> Result<(), LearnError> {
        self.load_with(task, lesson, interview, slot, false)
    }

    /// `keep`: add the task below the work already in the book and scroll to it, instead of
    /// opening a fresh book — the live-data step continues the drill it follows.
    fn load_with(
        &mut self,
        task: Task,
        lesson: Option<Lesson>,
        interview: Option<Interview>,
        slot: Option<Slot>,
        keep: bool,
    ) -> Result<(), LearnError> {
        let start_fields = task_start_fields(&task, lesson.as_ref(), interview.as_ref(), slot.as_ref());
        self.supply = None;
        if keep {
            task.setup.extend(&mut self.sheet.session, &task.focus)?;
            // Scroll only as far as needed to show the whole new table, so the solved drill
            // stays in view when both fit (about 24 rows are visible at the default zoom).
            let rows = task.setup.cells.iter().filter_map(|(c, ..)| gridcraft_engine::core::CellRef::parse(c)).map(|a| a.row);
            let (top, bottom) = rows.fold((u32::MAX, 0), |(lo, hi), r| (lo.min(r), hi.max(r)));
            let first = bottom.saturating_sub(23).min(top.saturating_sub(2));
            let y = self.sheet.session.doc().ok().and_then(|d| d.wb.sheet(0).map(|sh| sh.row_top(first) as f32));
            if let (Some(y), Some(v)) = (y, self.sheet.view_mut()) {
                v.scroll = egui::vec2(0.0, y.max(0.0));
            }
        } else {
            self.sheet.session = task.session()?;
            self.sheet.views.clear();
        }
        self.sheet.editor = None;
        self.keys.clear();
        self.feedback = None;
        self.ai_result = None;
        self.show_hints = 0;
        self.typed.clear();
        self.profile.set_cursor(&task.spec.skill);
        self.active = Some(Active {
            task,
            lesson,
            interview,
            slot,
            started: instant(),
            before: 0.0,
            hints: 0,
            recorded: false,
            query: String::new(),
            sql_result: None,
            before_grade: None,
        });
        self.apply_zoom();
        self.graded_revision = self.revision();
        self.apply_ribbon();
        self.log_event("task.start", start_fields);
        Ok(())
    }

    /// Speed run: ten tasks against the clock (one skill, or a mix).
    pub fn start_sprint(&mut self, skill: Option<&str>) -> Result<(), LearnError> {
        let lesson = self.profile.plan_sprint(skill);
        self.streak_extended = false;
        self.screen = Screen::Lesson;
        self.sprint_clock = Some(instant());
        self.next_in_lesson(lesson)
    }

    pub fn sprint_elapsed(&self) -> f32 {
        self.sprint_clock.map(|c| c.elapsed().as_secs_f32()).unwrap_or(0.0)
    }

    /// "Я это знаю": every skill of the unit at level 3; clean passes skip ahead.
    pub fn start_testout(&mut self, unit: usize) -> Result<(), LearnError> {
        let lesson = self.profile.plan_testout(unit);
        if lesson.total == 0 {
            self.notice = Some(ts("Здесь всё уже на уровне «Сам» или выше.").into());
            return Ok(());
        }
        self.streak_extended = false;
        self.screen = Screen::Lesson;
        self.next_in_lesson(lesson)
    }

    pub fn start_lesson(&mut self, focus: Option<&str>) -> Result<(), LearnError> {
        let lesson = self.profile.plan_lesson(now(), focus);
        self.streak_extended = false;
        self.screen = Screen::Lesson;
        self.next_in_lesson(lesson)
    }

    /// The stage the saved bookmark points at, when it is a supply step.
    pub(crate) fn cursor_supply_stage(&self) -> Option<usize> {
        self.profile.supply_stage.map(usize::from).filter(|s| *s < supply::STAGES)
    }

    fn supply_complete(&self) -> bool {
        self.supply.as_ref().is_some_and(|r| r.complete)
    }

    /// Opens the single supply route at the saved cursor stage (stage 0 when there is none).
    /// `start_supply` always builds stage 0, so the active task is advanced app-side afterwards.
    pub fn open_supply(&mut self) -> Result<(), LearnError> {
        let stage = self.cursor_supply_stage().filter(|_| !self.supply_complete()).unwrap_or(0);
        self.start_supply()?;
        if stage == 0 {
            return Ok(());
        }
        // Every stage shares the same seeded layout, so the stage-0 workbook already holds the data.
        let seed = self.active.as_ref().map(|a| a.task.spec.layout);
        let seed = seed.unwrap_or_else(|| self.profile.seed());
        let task = supply::task(stage, seed);
        let focus = task.focus.clone();
        if let Some(a) = &mut self.active {
            a.task = task;
        }
        self.sheet.session.execute("selection.set", serde_json::json!({"range": focus}))?;
        self.graded_revision = self.revision();
        self.profile.supply_stage = Some(stage as u8);
        self.save()
    }

    /// The one primary "Продолжить": never lose the place.
    /// (a) an exercise is open → return to it; (b) otherwise the autopilot picks the next skill
    /// (remediation → due reviews → frontier). The supply route is a section reached on purpose
    /// via [`Self::open_supply`]; its own position still persists in `cursor`.
    pub fn continue_course(&mut self) {
        if self.active.is_some() {
            self.screen = if self.supply_stage().is_some() {
                Screen::Supply
            } else if self.active.as_ref().is_some_and(|a| a.task.mode == Mode::Drill) {
                Screen::Lesson
            } else {
                Screen::Case
            };
            return;
        }
        let r = self.start_lesson(None);
        self.handle(r);
    }

    fn next_in_lesson(&mut self, mut lesson: Lesson) -> Result<(), LearnError> {
        for _ in 0..16 {
            let Some(slot) = lesson.queue.pop_front() else {
                self.finish_lesson(lesson);
                return Ok(());
            };
            // Skills skipped in this lesson are not offered again in it.
            let slot = match slot {
                Slot::Frontier if !lesson.skipped.is_empty() => match self.profile.frontier_excluding(&lesson.skipped) {
                    Some(id) => Slot::Focus(id.into()),
                    None => continue,
                },
                Slot::Review(s) | Slot::Mix(s) | Slot::Retry(s) | Slot::Live(s) | Slot::Focus(s) if lesson.skipped.contains(&s) => continue,
                other => other,
            };
            // A skill already done unaided is reviewed straight on live data, without a clean warm-up.
            let slot = match slot {
                Slot::Review(s) | Slot::Mix(s)
                    if content::live::supports(&s)
                        && self.profile.level(&s) >= 3
                        && self.profile.live_ready(&s, now())
                        && (!self.profile.sprinting(now()) || self.profile.goal_reached())
                        && lesson.sprint.is_none() =>
                {
                    if !lesson.live_done.contains(&s) {
                        lesson.live_done.push(s.clone());
                    }
                    Slot::Live(s)
                }
                other => other,
            };
            let mut spec = self.profile.spec_for(&slot);
            // The live step right after its drill shares the book; the register opens below.
            let below = gridcraft_learn_core::sheet::Setup::last_row(&self.sheet.session) + 3;
            let keep = matches!(&slot, Slot::Live(s) if self.live_after.as_deref() == Some(s.as_str())) && below <= 200;
            self.live_after = None;
            if keep {
                spec = content::live::with_top(&spec, below);
            }
            match Task::generate(&spec) {
                Ok(task) => {
                    if lesson.sprint.is_some() {
                        lesson.goal_total += task.goal_seconds;
                    }
                    self.load_with(task, Some(lesson), None, Some(slot), keep)?;
                    return self.save();
                }
                Err(e) => self.notice = Some(tr(&format!("Упражнение пропущено: {e}"))),
            }
        }
        self.finish_lesson(lesson);
        Ok(())
    }

    fn finish_lesson(&mut self, lesson: Lesson) {
        let sprint = lesson.sprint.clone().map(|kind| {
            let seconds = self.sprint_elapsed() + lesson.misses as f32 * SPRINT_PENALTY;
            let previous = self.profile.sprint_best(&kind);
            let result =
                SprintResult { at: now(), kind: kind.clone(), tasks: lesson.done as u32, seconds, misses: lesson.misses, par: lesson.goal_total };
            let (medal, record, change) = self.profile.record_sprint(result, now());
            self.celebrate(&change);
            if record && previous.is_some() {
                self.toasts.push(Toast {
                    icon: "🏁".into(),
                    text: tr(&format!("Новый рекорд! −{:.1} с", previous.unwrap_or(seconds) - seconds)),
                    born: self.time(),
                });
            }
            SprintSummary { kind, seconds, misses: lesson.misses, medal, record, previous, par: lesson.goal_total }
        });
        self.sprint_clock = None;
        let perfect = lesson.done > 0 && lesson.first_try == lesson.done;
        let test_out = lesson.level_ups.iter().any(|l| l.starts_with('⏩'));
        let change = self.profile.finish_lesson(perfect, lesson.best_combo, now());
        self.celebrate(&change);
        self.summary = Some(Summary {
            sprint,
            perfect,
            gems: change.gems,
            test_out,
            xp: lesson.xp,
            accuracy: lesson.accuracy(),
            seconds: lesson.seconds,
            best_combo: lesson.best_combo,
            level_ups: lesson.level_ups.clone(),
            streak: self.profile.current_streak(now()),
            streak_extended: self.streak_extended,
        });
        self.active = None;
        self.feedback = None;
        self.screen = Screen::Done;
        self.done_since = Some(self.time());
        self.pending_confetti = true;
        self.cue(Cue::Complete);
        let r = self.save();
        self.handle(r);
    }

    pub fn start_case(&mut self, id: &str, interview: bool) -> Result<(), LearnError> {
        let best = self.profile.cases.get(id).map(|c| c.best).unwrap_or(0.0);
        let level = if interview {
            if best >= 0.8 { 4 } else { 3 }
        } else if best >= 0.8 {
            3
        } else {
            2
        };
        let seed = self.profile.seed();
        let mut task = Task::generate(&Spec::new(id, level, seed))?;
        let minutes = CASES.iter().find(|c| c.id == id).map(|c| c.minutes).unwrap_or(20);
        let iv = if interview {
            task.mode = Mode::Interview;
            task.hints.clear();
            Some(Interview { case: id.into(), minutes, started: now() })
        } else {
            None
        };
        self.load(task, None, iv, None)?;
        self.screen = Screen::Case;
        self.save()
    }

    /// Picks a case the learner has seen least, for a fresh interview.
    pub fn start_interview(&mut self) -> Result<(), LearnError> {
        let seed = self.profile.seed() as usize;
        let mut ids: Vec<(&str, u32)> = CASES
            .iter()
            .filter(|c| self.profile.case_open(c.id))
            .map(|c| (c.id, self.profile.cases.get(c.id).map(|p| p.runs).unwrap_or(0)))
            .collect();
        if ids.is_empty() {
            self.notice = Some("Сначала подготовьте навыки для кейса: доступные этапы показаны в маршруте и старом курсе.".into());
            return Ok(());
        }
        ids.sort_by_key(|(_, runs)| *runs);
        let least = ids.first().map(|x| x.1).unwrap_or(0);
        let pool: Vec<&str> = ids.iter().filter(|x| x.1 == least).map(|x| x.0).collect();
        let id = pool.get(seed % pool.len().max(1)).copied().unwrap_or("case.sales");
        self.start_case(id, true)
    }

    pub fn submit_case(&mut self, timed_out: bool) -> Result<(), LearnError> {
        let Some(a) = self.active.take() else { return Ok(()) };
        let eval = evaluate(&a.task, &self.sheet.session, &self.keys)?;
        let seconds = a.seconds();
        let interview = a.interview.as_ref().map(|i| i.minutes);
        let change = self.profile.record_case(&a.task, &eval, seconds, interview, now());
        if eval.score >= 0.999 {
            self.pending_confetti = true;
            self.cue(Cue::Complete);
        } else {
            self.cue(if eval.score >= 0.5 { Cue::Correct } else { Cue::Wrong });
        }
        self.log_event(
            "submit_case",
            serde_json::json!({"score": eval.score, "seconds": seconds, "interview": interview.is_some(), "timed_out": timed_out}),
        );
        self.report = Some(Report { task: a.task, eval, seconds, interview: interview.is_some(), xp: change.xp, timed_out });
        self.screen = Screen::Report;
        self.save()
    }

    /// Grades the current drill. The first evaluation sets XP and the staircase.
    pub fn check(&mut self) -> Result<(), LearnError> {
        let Some(a) = self.active.as_ref() else { return Ok(()) };
        let eval = if a.task.sql.is_some() { sql::evaluate(&a.task, &a.query)? } else { evaluate(&a.task, &self.sheet.session, &self.keys)? };
        self.graded_revision = self.revision();
        let passed = eval.passed();
        let seconds = a.seconds();
        let hints = a.hints;
        let first = !a.recorded;
        let failing: Vec<String> = eval.outcomes.iter().filter(|o| !o.passed).map(|o| o.label.clone()).collect();
        let combo = a.lesson.as_ref().map(|l| l.combo).unwrap_or(0);
        let test_out = matches!(a.slot, Some(Slot::TestOut(_)));
        let mut fb = Feedback {
            passed,
            eval,
            xp: 0,
            level_up: None,
            level_down: false,
            goal_reached: false,
            corrected: passed && !first,
            change: Change::default(),
            note: None,
        };
        if first {
            let task = a.task.clone();
            let base = content::live::base(&task.spec.skill).unwrap_or(&task.spec.skill).to_string();
            let before = BeforeGrade { progress: self.profile.skills.get(&base).cloned(), live: self.profile.live.get(&base).cloned(), skill: base };
            if let Some(a) = self.active.as_mut() {
                a.before_grade = Some(before);
            }
            let change = self.profile.record(&task, passed, seconds, hints, combo, test_out, now());
            fb.xp = change.xp;
            fb.level_up = change.level_up;
            fb.level_down = change.level_down;
            fb.goal_reached = change.goal_reached;
            self.streak_extended |= change.streak_extended;
            if let Some(a) = self.active.as_mut() {
                a.recorded = true;
                if let Some(l) = a.lesson.as_mut() {
                    l.xp += change.xp;
                    l.seconds += seconds;
                    if passed {
                        l.first_try += 1;
                        l.combo += 1;
                        l.best_combo = l.best_combo.max(l.combo);
                    } else {
                        l.combo = 0;
                        l.misses += 1;
                    }
                    if let Some(level) = change.level_up {
                        let mark = if change.fast_track { "⏩ " } else { "" };
                        l.level_ups.push(format!(
                            "{mark}{} → {}",
                            task.skill_name(),
                            ts(content::LEVELS.get(usize::from(level) - 1).copied().unwrap_or(""))
                        ));
                    }
                }
            }
            if passed {
                let t = self.time();
                let new_combo = combo + 1;
                if change.fast_track {
                    self.pop = Some(Pop { text: ts("⏩ УЖЕ ЗНАЕШЬ!").into(), color: egui::Color32::from_rgb(59, 130, 246), born: t });
                } else if matches!(new_combo, 3 | 5 | 10 | 15 | 20) {
                    self.pop =
                        Some(Pop { text: tr(&format!("🔥 КОМБО ×{new_combo}")), color: egui::Color32::from_rgb(249, 115, 22), born: t });
                } else if change.speed == Speed::Lightning {
                    self.pop = Some(Pop { text: ts("⚡ МОЛНИЯ!").into(), color: egui::Color32::from_rgb(245, 180, 0), born: t });
                } else if change.speed == Speed::Fast {
                    self.pop = Some(Pop { text: ts("💨 БЫСТРО!").into(), color: egui::Color32::from_rgb(16, 185, 129), born: t });
                }
                if change.level_up.is_some() || matches!(new_combo, 5 | 10 | 15 | 20) {
                    self.pending_confetti = true;
                }
            }
            self.celebrate(&change);
            if change.live_rest {
                // Second miss in a row on live data: open the full walkthrough instead of looping.
                if let Some(a) = self.active.as_mut() {
                    a.hints = a.task.hints.len() as u32;
                    self.show_hints = a.hints;
                }
                fb.note = Some(
                    ts("Разбор открыт в подсказках сверху: где ловушка и какая формула её ловит. Эта тема на грязных данных вернётся завтра.").into(),
                );
            }
            fb.change = change;
        }
        self.log_mistake(&fb, first, hints, seconds);
        let issues = self.formula_issues();
        self.log_event(
            "check",
            serde_json::json!({"passed": passed, "failing": failing, "seconds": seconds, "hints": hints, "first": first, "issues": issues}),
        );
        self.cue(if passed {
            if fb.level_up.is_some() {
                Cue::LevelUp
            } else if fb.change.speed == Speed::Lightning {
                Cue::Lightning
            } else {
                Cue::Correct
            }
        } else {
            Cue::Wrong
        });
        let sprint_pass = passed && self.active.as_ref().is_some_and(|a| a.lesson.as_ref().is_some_and(|l| l.sprint.is_some()));
        self.feedback = Some(fb);
        if sprint_pass {
            // Sprints flow: a correct answer moves straight to the next task.
            return self.advance();
        }
        self.save()
    }

    pub fn time(&self) -> f64 {
        self.clock.elapsed().as_secs_f64()
    }

    /// Toasts for badges, quests, gems and the streak; a fanfare for a badge.
    fn celebrate(&mut self, change: &Change) {
        let t = self.time();
        for id in &change.achievements {
            if let Some((icon, name, _)) = achievement(id) {
                self.toasts.push(Toast { icon: icon.into(), text: tr(&format!("Достижение: {}  +20 💎", ts(name))), born: t });
                self.pending_confetti = true;
            }
        }
        for q in &change.quests_done {
            self.toasts.push(Toast { icon: "✅".into(), text: tr(&format!("Задание дня: {q}")), born: t });
        }
        if change.goal_reached {
            self.toasts.push(Toast { icon: "🎯".into(), text: ts("Цель дня выполнена!  +10 💎").into(), born: t });
        }
        if change.freeze_used {
            self.toasts.push(Toast {
                icon: "🧊".into(), text: ts("Заморозка спасла серию за пропущенный день").into(), born: t
            });
        }
        if let Some(w) = change.wager_won {
            self.toasts.push(Toast { icon: "🎲".into(), text: tr(&format!("Ставка выиграна!  +{w} 💎")), born: t });
        }
        if change.wager_lost {
            self.toasts
                .push(Toast { icon: "💔".into(), text: ts("Серия прервалась — ставка сгорела").into(), born: t });
        }
        if !change.achievements.is_empty() {
            self.cue(Cue::Achievement);
        }
        self.toasts.retain(|x| t - x.born < 6.0);
    }

    /// Runs the SQL query; a correct result is graded at once.
    pub fn run_sql(&mut self) {
        let Some(a) = self.active.as_mut() else { return };
        let Some(task_sql) = a.task.sql.as_ref() else { return };
        let result = sql::run(&task_sql.tables, &a.query);
        let correct = match (&result, sql::run(&task_sql.tables, &task_sql.reference)) {
            (Ok(got), Ok(want)) => sql::same(got, &want, task_sql.ordered).is_ok(),
            _ => false,
        };
        a.sql_result = Some(result);
        if correct && self.feedback.is_none() {
            let r = self.check();
            self.handle(r);
        }
    }

    /// Starts a timed multiple-choice test.
    pub fn start_blitz(&mut self, track: Track) {
        let seed = self.profile.seed();
        let questions = quiz::blitz(seed, track, 15);
        self.blitz = Some(Blitz { track, questions, index: 0, started: instant(), began: instant(), answers: Vec::new(), combo: 0 });
        self.screen = Screen::Blitz;
        self.log_event("blitz_start", serde_json::json!({"track": format!("{track:?}"), "seed": seed, "questions": 15}));
    }

    /// Answers the current blitz question (`None` = time ran out).
    pub fn blitz_answer(&mut self, choice: Option<usize>) {
        let t = self.time();
        let Some(b) = self.blitz.as_mut() else { return };
        if b.answered() {
            return;
        }
        let Some(q) = b.current() else { return };
        let used = b.used();
        let right = choice == Some(q.correct);
        let fast = right && used <= q.seconds as f32 * 0.35;
        let pts = quiz::points(right, used, q.seconds) + if right { b.combo.min(5) * 2 } else { 0 };
        b.combo = if right { b.combo + 1 } else { 0 };
        let combo = b.combo;
        b.answers.push((choice, used, pts));
        if right {
            if matches!(combo, 3 | 5 | 10) {
                self.pop = Some(Pop { text: tr(&format!("🔥 КОМБО ×{combo}")), color: egui::Color32::from_rgb(249, 115, 22), born: t });
            } else if fast {
                self.pop = Some(Pop { text: ts("⚡ МОЛНИЯ!").into(), color: egui::Color32::from_rgb(245, 180, 0), born: t });
            }
        }
        self.cue(if !right {
            Cue::Wrong
        } else if fast {
            Cue::Lightning
        } else {
            Cue::Correct
        });
        self.log_event(
            "blitz_answer",
            serde_json::json!({"choice": choice, "correct": right, "used": used, "points": pts, "modality": "mouse_or_key"}),
        );
    }

    pub fn blitz_next(&mut self) {
        let Some(b) = self.blitz.as_mut() else { return };
        if !b.answered() {
            return;
        }
        b.index += 1;
        b.started = instant();
        if b.index >= b.questions.len() {
            self.finish_blitz();
        }
    }

    fn finish_blitz(&mut self) {
        let Some(b) = self.blitz.take() else { return };
        let missed: Vec<String> = b.answers.iter().zip(&b.questions).filter(|(a, q)| a.0 != Some(q.correct)).map(|(_, q)| q.skill.clone()).collect();
        let result = BlitzResult {
            at: now(),
            track: Some(b.track),
            correct: b.correct(),
            total: b.questions.len() as u32,
            seconds: b.began.elapsed().as_secs_f32(),
            points: b.points(),
        };
        let change = self.profile.record_blitz(result, &missed, now());
        self.celebrate(&change);
        if b.correct() * 10 >= b.questions.len() as u32 * 8 {
            self.pending_confetti = true;
        }
        self.cue(Cue::Complete);
        self.log_event(
            "blitz_finish",
            serde_json::json!({"correct": b.correct(), "total": b.questions.len(), "points": b.points(), "track": format!("{:?}", b.track)}),
        );
        self.blitz_result = Some((b, change));
        self.screen = Screen::BlitzReport;
        let r = self.save();
        self.handle(r);
    }

    /// Writes or removes the daily reminder (systemd user timer).
    pub fn set_reminder(&mut self, at: Option<(u8, u8)>) {
        self.profile.remind = at;
        self.remind_status = Some(match remind::install(at) {
            Ok(m) => m,
            Err(e) => e,
        });
        let r = self.save();
        self.handle(r);
    }

    /// "Дальше": a missed exercise comes back at the end of the lesson, one level easier.
    pub fn advance(&mut self) -> Result<(), LearnError> {
        let Some(a) = self.active.take() else { return Ok(()) };
        let passed = self.feedback.as_ref().is_some_and(|f| f.passed);
        // «Пропустить» before any check is a skip, not forgetting: no grade, no FSRS lapse, no
        // retry; the skill is set aside for the rest of this lesson.
        let skipped = !a.recorded;
        self.log_event("advance", serde_json::json!({"passed": passed, "skill": a.task.spec.skill, "seconds": a.seconds()}));
        self.feedback = None;
        let Some(mut lesson) = a.lesson else {
            self.screen = Screen::Home;
            return self.save();
        };
        lesson.done += 1;
        let skill = a.task.spec.skill.clone();
        let live = content::live::base(&skill).map(str::to_string);
        if skipped {
            let base = live.clone().unwrap_or_else(|| skill.clone());
            if !lesson.skipped.contains(&base) {
                lesson.skipped.push(base);
            }
            return self.next_in_lesson(lesson);
        }
        // A live register resting after two misses does not come back in this lesson.
        let resting = live.as_deref().is_some_and(|b| !self.profile.live_ready(b, now()));
        if !passed && lesson.retries < 3 && !resting {
            lesson.retries += 1;
            // A missed register comes back as a fresh register at the same dirt tier.
            lesson.queue.push_back(match &live {
                Some(base) => Slot::Live(base.clone()),
                None => Slot::Retry(skill.clone()),
            });
            lesson.total += 1;
        }
        // A passed drill continues at once on hand-typed data, below it in the same book.
        let eligible = passed
            && live.is_none()
            && lesson.sprint.is_none()
            && !matches!(a.slot, Some(Slot::TestOut(_)))
            && content::live::supports(&skill)
            // Hidden entry errors only once the clean skill is solid («Сам»): earlier they just
            // stack two difficulties and most registers fail.
            && self.profile.level(&skill) >= 3
            && self.profile.live_ready(&skill, now())
            // During the preparation sprint every task moves toward the goal; live data waits
            // until the goal is reached, then it is confidence practice.
            && (!self.profile.sprinting(now()) || self.profile.goal_reached())
            && !lesson.live_done.contains(&skill);
        if eligible {
            lesson.live_done.push(skill.clone());
            lesson.queue.push_front(Slot::Live(skill.clone()));
            lesson.total += 1;
            self.live_after = Some(skill);
        }
        self.next_in_lesson(lesson)
    }

    /// Puts the exercise back as it was given, after the source got scrambled. Time and hints
    /// already used stay: a fresh start, not a free retry.
    pub fn reset_task(&mut self) -> Result<(), LearnError> {
        let Some(task) = self.active.as_ref().map(|a| a.task.clone()) else { return Ok(()) };
        if task.sql.is_some() {
            return Ok(());
        }
        self.sheet.session = task.session()?;
        self.sheet.views.clear();
        self.sheet.editor = None;
        self.keys.clear();
        self.feedback = None;
        self.apply_zoom();
        self.graded_revision = self.revision();
        self.apply_ribbon();
        self.log_event("reset", serde_json::json!({"skill": task.spec.skill}));
        Ok(())
    }

    /// «Мой ответ верный»: the learner disputes a miss. The grade is undone (no penalty, no
    /// credit either), the case goes to the mistakes journal for the grader to be fixed, and the
    /// lesson moves on without queueing a retry.
    pub fn dispute(&mut self) -> Result<(), LearnError> {
        if !self.feedback.as_ref().is_some_and(|f| !f.passed) {
            return Ok(());
        }
        let (hints, seconds) = self.active.as_ref().map(|a| (a.hints, a.seconds())).unwrap_or_default();
        if let Some(fb) = self.feedback.take() {
            self.log_mistake_as(&fb, "disputed", hints, seconds);
        }
        if let Some(b) = self.active.as_ref().and_then(|a| a.before_grade.clone()) {
            match b.progress {
                Some(p) => {
                    self.profile.skills.insert(b.skill.clone(), p);
                }
                None => {
                    self.profile.skills.remove(&b.skill);
                }
            }
            match b.live {
                Some(l) => {
                    self.profile.live.insert(b.skill.clone(), l);
                }
                None => {
                    self.profile.live.remove(&b.skill);
                }
            }
        }
        self.log_event("dispute", serde_json::json!({"skill": self.active.as_ref().map(|a| a.task.spec.skill.clone())}));
        self.feedback = None;
        let Some(a) = self.active.take() else { return Ok(()) };
        let Some(mut lesson) = a.lesson else {
            self.screen = Screen::Home;
            return self.save();
        };
        lesson.done += 1;
        if let Some(l) = lesson.misses.checked_sub(1) {
            lesson.misses = l;
        }
        self.next_in_lesson(lesson)
    }

    pub fn fix(&mut self) {
        self.feedback = None;
        self.graded_revision = self.revision();
        self.log_event("fix", serde_json::json!({}));
    }

    pub fn hint(&mut self) {
        let mut used = None;
        if let Some(a) = self.active.as_mut() {
            let max = a.task.hints.len() as u32;
            if a.hints < max {
                a.hints += 1;
            }
            self.show_hints = a.hints;
            used = Some(a.hints);
        }
        if let Some(hints) = used {
            self.log_event("hint", serde_json::json!({"hints": hints}));
        }
    }

    pub fn quit_to_home(&mut self) {
        if let Some(a) = self.active.take()
            && let Some(lesson) = a.lesson
            && lesson.done > 0
        {
            self.notice = Some(tr(&format!("Урок прерван: пройдено {} из {}.", lesson.done, lesson.total)));
        }
        self.feedback = None;
        self.sprint_clock = None;
        self.screen = Screen::Home;
        let r = self.save();
        self.handle(r);
    }

    /// Auto-check: once every answer cell is filled (or an action changed the sheet).
    fn auto_check(&mut self) {
        if self.screen != Screen::Lesson || self.feedback.is_some() || self.sheet.editor.is_some() {
            return;
        }
        let rev = self.revision();
        if rev.is_none() || rev == self.graded_revision {
            return;
        }
        let Some(a) = self.active.as_ref() else { return };
        if a.task.sql.is_some() {
            return;
        }
        let cells = a.task.answer_cells();
        if cells.is_empty() {
            match evaluate(&a.task, &self.sheet.session, &self.keys) {
                Ok(e) if e.passed() => {
                    let r = self.check();
                    self.handle(r);
                }
                _ => self.graded_revision = rev,
            }
            return;
        }
        let filled = cells
            .iter()
            .all(|c| gridcraft_learn_core::sheet::value(&self.sheet.session, c).is_ok_and(|v| !matches!(v, gridcraft_engine::core::Value::Empty)));
        if filled {
            let r = self.check();
            self.handle(r);
        }
    }

    pub fn logic(&mut self, ctx: &egui::Context) {
        self.observe_screen();
        self.observe_supply();
        if self.screen != Screen::Supply || !self.supply.as_ref().is_some_and(|r| r.paused || r.complete) {
            self.sheet.logic(ctx);
        }
        if let Some((seed, rx)) = &self.ai_pending
            && let Ok(result) = rx.try_recv()
        {
            if self.active.as_ref().is_some_and(|a| a.task.spec.layout == *seed) {
                self.ai_result = Some(result.unwrap_or_else(|e| e));
            }
            self.ai_pending = None;
        }
        // Blitz: an unanswered question times out.
        if self.screen == Screen::Blitz
            && let Some(b) = &self.blitz
            && !b.answered()
            && b.current().is_some_and(|q| b.used() >= q.seconds as f32)
        {
            self.blitz_answer(None);
        }
        // Interview timer.
        if self.screen == Screen::Case
            && let Some(a) = &self.active
            && let Some(iv) = &a.interview
            && a.seconds() >= iv.minutes as f32 * 60.0
        {
            let r = self.submit_case(true);
            self.handle(r);
        }
        if self.last_save.elapsed() > Duration::from_secs(3)
            && let Err(e) = self.save()
        {
            self.notice = Some(tr(&format!("Ошибка сохранения: {e}")));
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    /// Records shortcuts and takes Enter for the learning flow when a banner is up.
    pub fn raw_input_hook(&mut self, raw: &mut egui::RawInput) {
        if self.screen == Screen::Supply && self.supply.as_ref().is_some_and(|r| r.paused || r.complete) {
            return;
        }
        self.log_raw_input(raw);
        let mut taken = Vec::new();
        let in_task = matches!(self.screen, Screen::Lesson | Screen::Case | Screen::Supply);
        let editing = self.sheet.editor.is_some();
        for (i, e) in raw.events.iter().enumerate() {
            let egui::Event::Key { key, physical_key, pressed: true, modifiers, .. } = e else { continue };
            if in_task
                && let Some(c) = combo(*key, *physical_key, *modifiers)
                && !self.keys.contains(&c)
            {
                self.keys.push(c);
            }
            if *key == egui::Key::Enter {
                let plain = !modifiers.any();
                let ctrl = modifiers.command || modifiers.ctrl;
                match self.screen {
                    Screen::Home | Screen::Done if plain && !self.show_settings => {
                        self.enter = true;
                        taken.push(i);
                    }
                    Screen::Lesson if plain && self.feedback.is_some() => {
                        self.enter = true;
                        taken.push(i);
                    }
                    Screen::Lesson if ctrl && !editing => {
                        self.ctrl_enter = true;
                        taken.push(i);
                    }
                    Screen::Blitz | Screen::BlitzReport if plain => {
                        self.enter = true;
                        taken.push(i);
                    }
                    _ => {}
                }
            }
            if self.screen == Screen::Blitz && !modifiers.any() {
                let n = match key {
                    egui::Key::Num1 => Some(0),
                    egui::Key::Num2 => Some(1),
                    egui::Key::Num3 => Some(2),
                    egui::Key::Num4 => Some(3),
                    _ => None,
                };
                if n.is_some() {
                    self.blitz_key = n;
                    taken.push(i);
                }
            }
        }
        for i in taken.into_iter().rev() {
            if i < raw.events.len() {
                raw.events.remove(i);
            }
        }
        self.sheet.raw_input_hook(raw);
    }

    /// Local-only capture of raw input, tagged with its modality. Runs before the sheet so it
    /// sees exactly what the learner did; never consumes or alters the events.
    fn log_raw_input(&mut self, raw: &egui::RawInput) {
        if !self.events.is_enabled() {
            return;
        }
        let cell = self.current_cell();
        let mut events: Vec<(&str, serde_json::Value)> = Vec::new();
        for e in &raw.events {
            match e {
                egui::Event::Key { key, physical_key, pressed, modifiers, repeat } => {
                    events.push((
                        "key",
                        serde_json::json!({
                            "key": key.name(),
                            "physical": physical_key.as_ref().map(|k| k.name()),
                            "modifiers": modifiers_label(*modifiers),
                            "pressed": pressed,
                            "repeat": repeat,
                            "cell": cell,
                            "modality": "keyboard",
                        }),
                    ));
                    if *pressed {
                        let ctrl = modifiers.ctrl || modifiers.command;
                        if ctrl && *key == egui::Key::Z {
                            events.push((if modifiers.shift { "redo" } else { "undo" }, serde_json::json!({"modality": "keyboard"})));
                        } else if ctrl && *key == egui::Key::Y {
                            events.push(("redo", serde_json::json!({"modality": "keyboard"})));
                        }
                        if self.sheet.editor.is_some() && matches!(*key, egui::Key::Enter | egui::Key::Tab) {
                            let text = std::mem::take(&mut self.typed);
                            events.push(("text_commit", serde_json::json!({"cell": cell, "text": text, "modality": "keyboard"})));
                        }
                    }
                }
                egui::Event::Text(t) => {
                    self.typed.push_str(t);
                    events.push(("text", serde_json::json!({"text": t, "cell": cell, "modality": "keyboard"})));
                }
                egui::Event::Paste(t) => events.push(("paste", serde_json::json!({"text": t, "cell": cell, "modality": "clipboard"}))),
                egui::Event::Copy => events.push(("copy", serde_json::json!({"cell": cell, "modality": "clipboard"}))),
                egui::Event::Cut => events.push(("cut", serde_json::json!({"cell": cell, "modality": "clipboard"}))),
                egui::Event::PointerButton { pos, button, pressed, modifiers } => {
                    self.pointer_down = *pressed;
                    events.push((
                        "pointer",
                        serde_json::json!({
                            "x": pos.x,
                            "y": pos.y,
                            "button": format!("{button:?}"),
                            "pressed": pressed,
                            "modifiers": modifiers_label(*modifiers),
                            "cell": cell,
                            "modality": "mouse",
                        }),
                    ));
                }
                egui::Event::PointerMoved(pos) if self.pointer_down => {
                    events.push(("drag", serde_json::json!({"x": pos.x, "y": pos.y, "cell": cell, "modality": "mouse"})));
                }
                _ => {}
            }
        }
        for (kind, fields) in events {
            self.log_event(kind, fields);
        }
    }

    fn ask_ai(&mut self) {
        if !self.ai_consent
            || self.ai_pending.is_some()
            || self.ai_requests >= 20
            || self.ai_last.is_some_and(|t| t.elapsed() < Duration::from_secs(60))
        {
            return;
        }
        let (Some(config), Some(a)) = (self.ai_config.clone(), self.active.as_ref()) else { return };
        match ai::context(&a.task, &self.sheet.session) {
            Ok(context) => {
                let (tx, rx) = std::sync::mpsc::channel();
                let seed = a.task.spec.layout;
                std::thread::spawn(move || {
                    let _ = tx.send(config.request(context));
                });
                self.ai_pending = Some((seed, rx));
                self.ai_requests = self.ai_requests.saturating_add(1);
                self.ai_last = Some(instant());
            }
            Err(e) => self.ai_result = Some(e),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let enter = std::mem::take(&mut self.enter);
        let ctrl_enter = std::mem::take(&mut self.ctrl_enter);
        match self.screen {
            Screen::Supply => supply::show(self, ui),
            Screen::Home => {
                if enter {
                    self.continue_course();
                }
                screens::home(self, ui);
            }
            Screen::Lesson => {
                if enter && self.feedback.is_some() {
                    let r = self.advance();
                    self.handle(r);
                } else if ctrl_enter && self.feedback.is_none() {
                    if self.active.as_ref().is_some_and(|a| a.task.sql.is_some()) {
                        self.run_sql();
                    } else {
                        let r = self.check();
                        self.handle(r);
                    }
                }
                if self.screen == Screen::Lesson {
                    screens::lesson(self, ui);
                    self.auto_check();
                }
            }
            Screen::Case => screens::case(self, ui),
            Screen::Done => {
                if enter {
                    let r = self.start_lesson(None);
                    self.handle(r);
                }
                screens::done(self, ui);
            }
            Screen::Report => screens::report(self, ui),
            Screen::Stats => screens::stats(self, ui),
            Screen::Blitz => {
                if let Some(k) = self.blitz_key.take() {
                    if self.blitz.as_ref().is_some_and(|b| b.answered()) {
                        self.blitz_next();
                    } else {
                        self.blitz_answer(Some(k));
                    }
                } else if enter {
                    self.blitz_next();
                }
                if self.screen == Screen::Blitz {
                    screens::blitz(self, ui);
                }
            }
            Screen::BlitzReport => {
                if enter {
                    self.screen = Screen::Home;
                }
                screens::blitz_report(self, ui);
            }
            Screen::Ledger => {
                let p = view::palette(self.profile.dark);
                let path = self.path.as_ref().map(|p| p.with_file_name("ledger.json"));
                let ledger = self.ledger.get_or_insert_with(|| ledger_ui::LedgerView::load(path));
                let seed = self.profile.seed();
                let out = ledger_ui::show(ledger, ui, &p, seed);
                if out.back {
                    self.screen = Screen::Home;
                }
                if let Some((secs, goal)) = out.solved {
                    let change = self.profile.record_ledger_task(secs, goal, now());
                    let t = self.time();
                    let text = match change.speed {
                        Speed::Lightning => ts("⚡ МОЛНИЯ!"),
                        Speed::Fast => ts("💨 БЫСТРО!"),
                        Speed::Normal => ts("✔ ВЕРНО!"),
                    };
                    self.pop = Some(Pop { text: text.into(), color: egui::Color32::from_rgb(16, 185, 129), born: t });
                    self.cue(if change.speed == Speed::Lightning { Cue::Lightning } else { Cue::Correct });
                    self.celebrate(&change);
                    let r = self.save();
                    self.handle(r);
                }
                if out.failed {
                    self.cue(Cue::Wrong);
                }
                if let Some((solved, total)) = out.finished {
                    if solved * 10 >= total * 8 {
                        self.pending_confetti = true;
                    }
                    self.cue(Cue::Complete);
                }
            }
        }
        if self.show_settings {
            screens::settings(self, &ctx);
        }
        if std::mem::take(&mut self.pending_confetti) {
            let area = ctx.content_rect();
            view::spawn_confetti(&mut self.confetti, area, now());
        }
        view::draw_confetti(&ctx, &mut self.confetti);
        let t = self.time();
        if self.pop.as_ref().is_some_and(|p| t - p.born > 1.4) {
            self.pop = None;
        }
        if let Some(p) = &self.pop {
            view::draw_pop(&ctx, &p.text, p.color, ((t - p.born) / 1.4) as f32);
        }
        self.toasts.retain(|x| t - x.born < 5.0);
        view::draw_toasts(&ctx, &self.toasts, t, &view::palette(self.sheet.ui.dark));
        if self.pop.is_some() || !self.toasts.is_empty() {
            ctx.request_repaint();
        }
    }
}

/// "Ctrl+Shift+L", "Alt+=", "F4", "Ctrl+Down"… Plain typing is not recorded.
pub fn combo(key: egui::Key, physical: Option<egui::Key>, m: egui::Modifiers) -> Option<String> {
    use egui::Key as K;
    let k = match physical {
        Some(p) if p.name().len() == 1 || matches!(p, K::Semicolon | K::Equals | K::Plus) => p,
        _ => key,
    };
    let fkey = matches!(k, K::F1 | K::F2 | K::F3 | K::F4 | K::F5 | K::F6 | K::F7 | K::F8 | K::F9 | K::F10 | K::F11 | K::F12);
    let ctrl = m.ctrl || m.command;
    if !ctrl && !m.alt && !fkey {
        return None;
    }
    let name = match k {
        K::ArrowDown => "Down".to_string(),
        K::ArrowUp => "Up".to_string(),
        K::ArrowLeft => "Left".to_string(),
        K::ArrowRight => "Right".to_string(),
        K::Equals | K::Plus => "=".to_string(),
        K::Semicolon => ";".to_string(),
        other => other.name().to_string(),
    };
    let mut s = String::new();
    if ctrl {
        s.push_str("Ctrl+");
    }
    if m.shift {
        s.push_str("Shift+");
    }
    if m.alt {
        s.push_str("Alt+");
    }
    s.push_str(&name);
    Some(s)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), LearnError> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| LearnError::Invalid(e.to_string()))?;
    }
    let temp = path.with_extension("json.tmp");
    let mut file = std::fs::File::create(&temp).map_err(|e| LearnError::Invalid(e.to_string()))?;
    file.write_all(bytes).map_err(|e| LearnError::Invalid(e.to_string()))?;
    file.sync_all().map_err(|e| LearnError::Invalid(e.to_string()))?;
    std::fs::rename(&temp, path).map_err(|e| LearnError::Invalid(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests;
