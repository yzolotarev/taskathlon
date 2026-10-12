//! Taskathlon: open the app, press one button, practise in a real spreadsheet.
#![forbid(unsafe_code)]
#![cfg(not(target_arch = "wasm32"))]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented, clippy::unreachable)]

mod ai;
pub mod ledger_ui;
pub mod remind;
mod screens;
pub mod sound;
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
    Lesson,
    Done,
    Case,
    Report,
    Stats,
    Blitz,
    BlitzReport,
    Ledger,
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
    pub sheet: SheetApp,
    pub profile: Profile,
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
}

impl LearningApp {
    pub fn new(path: Option<PathBuf>) -> Result<Self, LearnError> {
        let mut notice = None;
        let save = match path.as_ref() {
            Some(p) if p.exists() => {
                let bytes = std::fs::read(p).map_err(|e| LearnError::Invalid(format!("Не удалось прочитать {}: {e}", p.display())))?;
                match Save::decode(&bytes) {
                    Ok(s) => Some(s),
                    Err(_) => {
                        // An older or damaged file is kept aside, never overwritten.
                        let backup = p.with_extension("old.json");
                        std::fs::rename(p, &backup).map_err(|e| LearnError::Invalid(format!("Не удалось сохранить копию старого прогресса: {e}")))?;
                        notice = Some(format!("Старый файл прогресса сохранён как {}. Начинаем новую версию тренажёра.", backup.display()));
                        None
                    }
                }
            }
            _ => None,
        };
        let (profile, current) = save.map(|s| (s.profile, s.current)).unwrap_or_default();
        let mut sheet = SheetApp::new(gridcraft_engine::Session::new(), Services::default());
        sheet.ui.dark = profile.dark;
        sheet.ui.language = gridcraft_ui_egui::i18n::Language::Ru;
        sheet.ui.ribbon_collapsed = true;
        sheet.ui.animations = false;
        let mut app = Self {
            sheet,
            profile,
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
        };
        app.greeting = Some(remind::greeting(&app.profile, now()));
        if let Some(c) = current {
            app.sheet.session = c.session();
            app.keys = c.keys.clone();
            app.screen = if c.task.mode == Mode::Drill { Screen::Lesson } else { Screen::Case };
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
            });
            app.graded_revision = app.revision();
            app.apply_ribbon();
        }
        Ok(app)
    }

    fn revision(&self) -> Option<u64> {
        self.sheet.session.doc().ok().map(|d| d.revision)
    }

    pub fn save(&mut self) -> Result<(), LearnError> {
        self.last_save = instant();
        let Some(path) = self.path.clone() else { return Ok(()) };
        let current = match &self.active {
            Some(a) => Some(Current {
                task: a.task.clone(),
                workbook: (*self.sheet.session.doc()?.wb).clone(),
                lesson: a.lesson.clone(),
                interview: a.interview.clone(),
                elapsed: a.seconds(),
                hints: a.hints,
                keys: self.keys.clone(),
                query: a.query.clone(),
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
        let session = task.session()?;
        self.sheet.session = session;
        self.sheet.views.clear();
        self.sheet.editor = None;
        self.keys.clear();
        self.feedback = None;
        self.ai_result = None;
        self.show_hints = 0;
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
        });
        self.graded_revision = self.revision();
        self.apply_ribbon();
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
            self.notice = Some("Здесь всё уже на уровне «Сам» или выше.".into());
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

    fn next_in_lesson(&mut self, mut lesson: Lesson) -> Result<(), LearnError> {
        for _ in 0..4 {
            let Some(slot) = lesson.queue.pop_front() else {
                self.finish_lesson(lesson);
                return Ok(());
            };
            let spec = self.profile.spec_for(&slot);
            match Task::generate(&spec) {
                Ok(task) => {
                    if lesson.sprint.is_some() {
                        lesson.goal_total += task.goal_seconds;
                    }
                    self.load(task, Some(lesson), None, Some(slot))?;
                    return self.save();
                }
                Err(e) => self.notice = Some(format!("Упражнение пропущено: {e}")),
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
                    text: format!("Новый рекорд! −{:.1} с", previous.unwrap_or(seconds) - seconds),
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
        let mut ids: Vec<(&str, u32)> = CASES.iter().map(|c| (c.id, self.profile.cases.get(c.id).map(|p| p.runs).unwrap_or(0))).collect();
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
        };
        if first {
            let task = a.task.clone();
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
                            content::LEVELS.get(usize::from(level) - 1).copied().unwrap_or("")
                        ));
                    }
                }
            }
            if passed {
                let t = self.time();
                let new_combo = combo + 1;
                if change.fast_track {
                    self.pop = Some(Pop { text: "⏩ УЖЕ ЗНАЕШЬ!".into(), color: egui::Color32::from_rgb(59, 130, 246), born: t });
                } else if matches!(new_combo, 3 | 5 | 10 | 15 | 20) {
                    self.pop = Some(Pop { text: format!("🔥 КОМБО ×{new_combo}"), color: egui::Color32::from_rgb(249, 115, 22), born: t });
                } else if change.speed == Speed::Lightning {
                    self.pop = Some(Pop { text: "⚡ МОЛНИЯ!".into(), color: egui::Color32::from_rgb(245, 180, 0), born: t });
                } else if change.speed == Speed::Fast {
                    self.pop = Some(Pop { text: "💨 БЫСТРО!".into(), color: egui::Color32::from_rgb(16, 185, 129), born: t });
                }
                if change.level_up.is_some() || matches!(new_combo, 5 | 10 | 15 | 20) {
                    self.pending_confetti = true;
                }
            }
            self.celebrate(&change);
            fb.change = change;
        }
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
                self.toasts.push(Toast { icon: icon.into(), text: format!("Достижение: {name}  +20 💎"), born: t });
                self.pending_confetti = true;
            }
        }
        for q in &change.quests_done {
            self.toasts.push(Toast { icon: "✅".into(), text: format!("Задание дня: {q}"), born: t });
        }
        if change.goal_reached {
            self.toasts.push(Toast { icon: "🎯".into(), text: "Цель дня выполнена!  +10 💎".into(), born: t });
        }
        if change.freeze_used {
            self.toasts.push(Toast {
                icon: "🧊".into(), text: "Заморозка спасла серию за пропущенный день".into(), born: t
            });
        }
        if let Some(w) = change.wager_won {
            self.toasts.push(Toast { icon: "🎲".into(), text: format!("Ставка выиграна!  +{w} 💎"), born: t });
        }
        if change.wager_lost {
            self.toasts.push(Toast { icon: "💔".into(), text: "Серия прервалась — ставка сгорела".into(), born: t });
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
                self.pop = Some(Pop { text: format!("🔥 КОМБО ×{combo}"), color: egui::Color32::from_rgb(249, 115, 22), born: t });
            } else if fast {
                self.pop = Some(Pop { text: "⚡ МОЛНИЯ!".into(), color: egui::Color32::from_rgb(245, 180, 0), born: t });
            }
        }
        self.cue(if !right {
            Cue::Wrong
        } else if fast {
            Cue::Lightning
        } else {
            Cue::Correct
        });
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
        if !a.recorded {
            self.profile.record(&a.task, false, a.seconds(), a.hints, 0, false, now());
        }
        self.feedback = None;
        let Some(mut lesson) = a.lesson else {
            self.screen = Screen::Home;
            return self.save();
        };
        lesson.done += 1;
        if !passed && lesson.retries < 3 {
            lesson.retries += 1;
            lesson.queue.push_back(Slot::Retry(a.task.spec.skill.clone()));
            lesson.total += 1;
        }
        self.next_in_lesson(lesson)
    }

    pub fn fix(&mut self) {
        self.feedback = None;
        self.graded_revision = self.revision();
    }

    pub fn hint(&mut self) {
        if let Some(a) = self.active.as_mut() {
            let max = a.task.hints.len() as u32;
            if a.hints < max {
                a.hints += 1;
            }
            self.show_hints = a.hints;
        }
    }

    pub fn quit_to_home(&mut self) {
        if let Some(a) = self.active.take()
            && let Some(lesson) = a.lesson
            && lesson.done > 0
        {
            self.notice = Some(format!("Урок прерван: пройдено {} из {}.", lesson.done, lesson.total));
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
        self.sheet.logic(ctx);
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
            self.notice = Some(format!("Ошибка сохранения: {e}"));
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    /// Records shortcuts and takes Enter for the learning flow when a banner is up.
    pub fn raw_input_hook(&mut self, raw: &mut egui::RawInput) {
        let mut taken = Vec::new();
        let in_task = matches!(self.screen, Screen::Lesson | Screen::Case);
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
            Screen::Home => {
                if enter {
                    let r = self.start_lesson(None);
                    self.handle(r);
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
                        Speed::Lightning => "⚡ МОЛНИЯ!",
                        Speed::Fast => "💨 БЫСТРО!",
                        Speed::Normal => "✔ ВЕРНО!",
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
