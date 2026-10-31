//! The learner model: a difficulty staircase per skill, FSRS reviews, XP and streak,
//! and the lesson planner that keeps every next step slightly harder than the last.

use crate::content::{CASES, SKILLS};
use crate::fsrs::{Card, Grade};
use crate::{Evaluation, Mode, Spec, Task};
use gridcraft_tr::{t as tr, ts};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

pub const DAY: u64 = 86_400;
/// Sprints always run at level 2 ("named tool"), so records stay comparable.
pub const SPRINT_LEVEL: u8 = 2;
pub const SPRINT_LEN: usize = 10;
/// A miss in a sprint adds this many seconds to the time.
pub const SPRINT_PENALTY: f32 = 10.0;
/// Clean successes needed at levels 1, 2, 3 before the next level.
const NEED: [u8; 3] = [2, 2, 3];
/// Nested lookup/error-handling is the current interview-prep ladder: keep massed practice
/// long enough for the typing pattern to become familiar before reducing scaffolding.
/// `sprint`: before a dated goal the lookup ladder uses the ordinary thresholds; its massed
/// repetitions are for the long course, not for the night before an interview.
fn successes_needed(id: &str, level: u8, sprint: bool) -> u8 {
    if !sprint && matches!(id, "lookup.error_observe" | "lookup.iferror.wrap" | "lookup.iferror.combine") {
        match level {
            1 | 2 => 5,
            3 => 8,
            _ => u8::MAX,
        }
    } else {
        NEED.get(usize::from(level.saturating_sub(1))).copied().unwrap_or(u8::MAX)
    }
}
/// Units each unit builds on (it opens when those are all at level 2+).
/// Skills a supply/procurement assistant needs first (job-interview priority), in teaching
/// order. The autopilot reaches each one through the path, never skipping its prerequisites.
const GOAL: &[&str] = &[
    "cond.sumif",
    "cond.countif",
    "supply.need",
    "lookup.vlookup",
    "check.reconcile",
    // Hidden spaces show up in live data from tier 2: learn to find and remove them first.
    "text.trim",
    "check.spaces",
    "data.filter",
    "data.pivot",
    "supply.overdue",
    "choose.totals",
    "lookup.error_observe",
    "lookup.iferror.combine",
    "choose.lookup",
    "check.catalog",
    "cond.sumifs",
];

/// The evening before the supply-assistant interview: once the goal skills are at «Сам», the
/// sprint's «Продолжить» walks these in order (pivots from several exports, then the working
/// week), one clean pass each, instead of cycling old reviews. Known basics → start at level 2.
const ROUTE: &[(&str, u8)] = &[
    // Pivots, smoothly: the steps by hand, rows × columns, then the manager's words.
    ("data.pivot", 1),
    ("data.pivot", 2),
    ("data.pivot", 3),
    ("orion.pivotread", 1),
    ("orion.pivotread", 2),
    ("data.pivot", 4),
    ("orion.pivotread", 3),
    ("orion.pivotread", 4),
    // Several exports into one list, then one pivot.
    ("asm.pivot", 1),
    ("asm.pivot", 2),
    ("asm.pivot", 3),
    ("asm.pivot", 4),
    ("orion.join", 1),
    ("orion.join", 3),
    // The working week.
    ("orion.open", 2),
    ("orion.overdue", 2),
    ("orion.contact", 2),
    ("orion.fx", 2),
    ("orion.payweek", 2),
    ("orion.prepay", 2),
    ("orion.reconcile", 2),
    ("orion.defects", 2),
    ("orion.mrp", 2),
    ("orion.moq", 2),
    ("orion.reorder", 2),
    ("orion.landed", 2),
    ("orion.compare", 2),
    ("orion.abc", 2),
    ("orion.exam", 3),
    // The real thing: a dozen sheets, several currencies, sales abroad.
    ("orion.quarter", 1),
    ("orion.quarter", 3),
];

const UNIT_NEEDS: [&[usize]; 14] = [&[], &[0], &[1], &[2], &[3], &[4], &[2], &[2], &[2], &[1], &[], &[], &[1], &[]];

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SkillProgress {
    /// 0 = not started; otherwise the current staircase level 1..=4.
    pub level: u8,
    pub clean: u8,
    pub misses: u8,
    pub attempts: u32,
    pub successes: u32,
    pub card: Card,
    pub best_seconds: f32,
    pub seconds: f32,
    pub hints: u32,
}

/// Live-data steps for one skill: how much entry dirt the next register carries.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LiveProgress {
    /// 1..=4; 0 in old saves means "not started" and reads as 1.
    pub tier: u8,
    pub clean: u8,
    pub misses: u8,
    pub attempts: u32,
    pub passes: u32,
    /// After two misses in a row the register rests until this time (unix seconds): the
    /// learner gets a walkthrough, and the skill comes back on live data the next day.
    #[serde(default)]
    pub rest_until: u64,
}

/// How long a register rests after two misses in a row: back the next day.
pub const LIVE_REST: u64 = 16 * 3600;

/// How far the learner is from the interview goal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GoalPlan {
    pub percent: u8,
    /// Correct, unassisted solutions still needed.
    pub passes_left: u32,
    /// With this learner's miss rate.
    pub tasks_left: u32,
    pub minutes_left: u32,
}

/// Unassisted live passes needed to add one more kind of dirt.
pub const LIVE_STEP: u8 = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attempt {
    pub skill: String,
    pub level: u8,
    pub at: u64,
    pub seconds: f32,
    pub hints: u32,
    pub passed: bool,
    pub mode: Mode,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CaseProgress {
    pub runs: u32,
    pub best: f32,
    pub last: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterviewResult {
    pub case: String,
    pub at: u64,
    pub score: f32,
    pub seconds: f32,
    pub minutes: u32,
    pub missed: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    /// Preparation sprint toward a dated goal (a job interview), unix seconds; 0 = off. Until
    /// then lessons serve only what moves toward the goal, then warm-up and confidence practice.
    pub sprint_until: u64,
    pub xp: u32,
    /// Local day number → XP earned that day.
    pub days: BTreeMap<u64, u32>,
    pub streak: u32,
    pub last_day: u64,
    pub goal: u32,
    /// Hours east of UTC for day boundaries (Moscow by default).
    pub utc_offset: i64,
    pub skills: BTreeMap<String, SkillProgress>,
    /// Observable performance, separate from memory scheduling and XP.
    pub evidence: BTreeMap<String, crate::evidence::SkillEvidence>,
    pub cases: BTreeMap<String, CaseProgress>,
    /// Skills a failed case or interview sent back for practice.
    pub remediation: Vec<String>,
    pub log: Vec<Attempt>,
    pub interviews: Vec<InterviewResult>,
    pub next_seed: u64,
    pub sound: bool,
    pub dark: bool,
    /// English interface instead of Russian.
    #[serde(default)]
    pub english: bool,
    /// The one course position: the last learned skill id (`lookup.vlookup`,
    /// `route.supply.3`, …). Persisted independently of the active exercise so the
    /// unified course resumes where the learner stopped.
    #[serde(default)]
    pub cursor: Option<String>,
    /// Section bookmark: the supply route's last stage, kept separately so entering the skill
    /// line does not lose the route position. App-managed; core only stores it.
    #[serde(default)]
    pub supply_stage: Option<u8>,
    /// Local-only action logging for later analysis; on unless the learner turns it off.
    #[serde(default = "default_true")]
    pub logging: bool,
    /// Interface size in percent (75..=200); restored on startup.
    #[serde(default = "default_100")]
    pub interface_scale: u16,
    /// Sheet zoom in percent; restored on startup.
    #[serde(default = "default_100")]
    pub sheet_zoom: u16,
    /// Currency for streak freezes and wagers.
    pub gems: u32,
    pub freezes: u8,
    pub frozen_days: Vec<u64>,
    pub wager: Option<Wager>,
    pub quest_day: u64,
    pub quests: Vec<Quest>,
    pub achievements: BTreeMap<String, u64>,
    pub lightning: u32,
    pub lessons: u32,
    pub perfect_lessons: u32,
    pub best_combo: u32,
    pub sql_solved: u32,
    pub blitz: Vec<crate::quiz::BlitzResult>,
    /// Daily reminder time (hour, minute), local.
    pub remind: Option<(u8, u8)>,
    pub sprints: Vec<SprintResult>,
    /// Live-data progress per practised skill id (not the `live.` task id).
    pub live: BTreeMap<String, LiveProgress>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SprintResult {
    pub at: u64,
    /// "mix" or a skill id.
    pub kind: String,
    pub tasks: u32,
    /// Wall time plus penalties.
    pub seconds: f32,
    pub misses: u32,
    pub par: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Medal {
    None,
    Bronze,
    Silver,
    Gold,
}

impl Medal {
    /// Against the sum of target times: gold under 40 %, silver 60 %, bronze 85 %.
    pub fn of(seconds: f32, par: f32) -> Self {
        let r = seconds / par.max(1.0);
        if r <= 0.4 {
            Self::Gold
        } else if r <= 0.6 {
            Self::Silver
        } else if r <= 0.85 {
            Self::Bronze
        } else {
            Self::None
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Gold => ts("🥇 Золото"),
            Self::Silver => ts("🥈 Серебро"),
            Self::Bronze => ts("🥉 Бронза"),
            Self::None => ts("Без медали"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Wager {
    pub start_day: u64,
    pub days: u32,
    pub stake: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestKind {
    Xp,
    Lightning,
    Perfect,
    Combo,
    Lessons,
    Sql,
    Blitz,
    Review,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quest {
    pub kind: QuestKind,
    pub target: u32,
    pub progress: u32,
    pub reward: u32,
    pub done: bool,
}

impl Quest {
    pub fn title(&self) -> String {
        let t = self.target;
        match self.kind {
            QuestKind::Xp => tr(&format!("Набери {t} XP")),
            QuestKind::Lightning => tr(&format!("Ответь молниеносно {t} раз")),
            QuestKind::Perfect => ts("Пройди урок без единой ошибки").into(),
            QuestKind::Combo => tr(&format!("Собери комбо ×{t}")),
            QuestKind::Lessons => tr(&format!("Пройди уроков: {t}")),
            QuestKind::Sql => tr(&format!("Реши SQL-задач: {t}")),
            QuestKind::Blitz => ts("Пройди экспресс-тест").into(),
            QuestKind::Review => tr(&format!("Повтори навыков: {t}")),
        }
    }
}

/// Badges: id, icon, name, how to get it.
pub const ACHIEVEMENTS: &[(&str, &str, &str, &str)] = &[
    ("first_lesson", "🎓", "Первый урок", "Пройди первый урок"),
    ("streak_3", "🔥", "Три дня", "Занимайся 3 дня подряд"),
    ("streak_7", "🔥", "Неделя", "Занимайся 7 дней подряд"),
    ("streak_30", "🏆", "Месяц", "Занимайся 30 дней подряд"),
    ("xp_500", "⚡", "500 XP", "Набери 500 XP"),
    ("xp_3000", "⚡", "3000 XP", "Набери 3000 XP"),
    ("lightning_10", "⚡", "Молния", "10 молниеносных ответов"),
    ("lightning_100", "🌩", "Гроза", "100 молниеносных ответов"),
    ("perfect_1", "💎", "Без ошибок", "Урок без единой ошибки"),
    ("perfect_10", "💎", "Перфекционист", "10 уроков без ошибок"),
    ("combo_10", "🎯", "Комбо ×10", "10 верных ответов подряд"),
    ("sql_10", "🗄", "SQL-старт", "Реши 10 SQL-задач"),
    ("sql_master", "🗄", "SQL-мастер", "Все SQL-навыки на уровне «Сам»"),
    ("interview_80", "💼", "Оффер близко", "Собеседование на 80%+"),
    ("blitz_80", "⏱", "Блиц", "Экспресс-тест на 80%+"),
    ("wager_won", "🎲", "Слово держу", "Выиграй ставку на серию"),
    ("sprint_record", "🏁", "Личный рекорд", "Побей свой рекорд в спринте"),
    ("sprint_gold", "🥇", "Золотой спринт", "Золото в спринте"),
];

pub fn achievement(id: &str) -> Option<(&'static str, &'static str, &'static str)> {
    ACHIEVEMENTS.iter().find(|a| a.0 == id).map(|a| (a.1, a.2, a.3))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Speed {
    #[default]
    Normal,
    Fast,
    Lightning,
}

fn default_true() -> bool {
    true
}

fn default_100() -> u16 {
    100
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            sprint_until: 0,
            xp: 0,
            days: BTreeMap::new(),
            streak: 0,
            last_day: 0,
            goal: 50,
            utc_offset: 3,
            skills: BTreeMap::new(),
            evidence: BTreeMap::new(),
            cases: BTreeMap::new(),
            remediation: Vec::new(),
            log: Vec::new(),
            interviews: Vec::new(),
            next_seed: 1,
            sound: true,
            dark: false,
            english: false,
            cursor: None,
            supply_stage: None,
            logging: true,
            interface_scale: 100,
            sheet_zoom: 100,
            gems: 50,
            freezes: 0,
            frozen_days: Vec::new(),
            wager: None,
            quest_day: 0,
            quests: Vec::new(),
            achievements: BTreeMap::new(),
            lightning: 0,
            lessons: 0,
            perfect_lessons: 0,
            best_combo: 0,
            sql_solved: 0,
            blitz: Vec::new(),
            remind: None,
            sprints: Vec::new(),
            live: BTreeMap::new(),
        }
    }
}

/// What a recorded answer changed, for the feedback banner.
#[derive(Clone, Debug, Default)]
pub struct Change {
    pub xp: u32,
    pub level_up: Option<u8>,
    pub level_down: bool,
    pub streak_extended: bool,
    pub goal_reached: bool,
    pub speed: Speed,
    pub speed_bonus: u32,
    pub combo_bonus: u32,
    /// Live data: the register now carries this many kinds of dirt.
    pub live_up: Option<u8>,
    /// The skill jumped ahead because it was clearly already known.
    pub fast_track: bool,
    /// Live data missed twice in a row: show the walkthrough; the register rests until tomorrow.
    pub live_rest: bool,
    pub gems: u32,
    pub achievements: Vec<String>,
    pub quests_done: Vec<String>,
    pub freeze_used: bool,
    pub wager_won: Option<u32>,
    pub wager_lost: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Slot {
    Frontier,
    Review(String),
    Mix(String),
    Retry(String),
    Focus(String),
    /// Skip-ahead check at level 3: a clean pass marks the skill as known.
    TestOut(String),
    /// Speed run at a fixed level so times are comparable.
    Sprint(String),
    /// The just-practised skill on a hand-typed register (skill id, not the `live.` id).
    Live(String),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Lesson {
    pub queue: VecDeque<Slot>,
    pub total: usize,
    pub done: usize,
    pub first_try: usize,
    pub xp: u32,
    pub combo: u32,
    pub best_combo: u32,
    pub retries: u8,
    pub seconds: f32,
    pub level_ups: Vec<String>,
    /// Speed run: "mix" or a skill id. Passes advance at once; the clock is what counts.
    #[serde(default)]
    pub sprint: Option<String>,
    /// Sum of the tasks' target times, for sprint medals.
    #[serde(default)]
    pub goal_total: f32,
    #[serde(default)]
    pub misses: u32,
    /// Skills that already continued on live data in this lesson (once each).
    #[serde(default)]
    pub live_done: Vec<String>,
    /// Skills the learner skipped in this lesson: not offered again until the next one.
    #[serde(default)]
    pub skipped: Vec<String>,
}

impl Lesson {
    pub fn progress(&self) -> f32 {
        self.done as f32 / self.total.max(1) as f32
    }
    pub fn finished(&self) -> bool {
        self.queue.is_empty()
    }
    pub fn accuracy(&self) -> f32 {
        self.first_try as f32 / self.done.max(1) as f32
    }
}

impl Profile {
    pub fn day(&self, now: u64) -> u64 {
        (now as i64 + self.utc_offset * 3600).max(0) as u64 / DAY
    }
    pub fn today_xp(&self, now: u64) -> u32 {
        self.days.get(&self.day(now)).copied().unwrap_or(0)
    }
    /// The streak as it stands today (broken if yesterday had no practice).
    pub fn current_streak(&self, now: u64) -> u32 {
        let today = self.day(now);
        // A missed day is covered while a streak freeze is in stock.
        if self.last_day + 1 >= today || (self.last_day + 2 == today && self.freezes > 0) { self.streak } else { 0 }
    }

    /// Today's practice is still missing and the streak would be lost tomorrow.
    pub fn streak_at_risk(&self, now: u64) -> bool {
        self.current_streak(now) > 0 && self.today_xp(now) == 0
    }
    pub fn level(&self, id: &str) -> u8 {
        self.skills.get(id).map(|s| s.level).unwrap_or(0)
    }

    /// Records the one course position (called when a task starts).
    pub fn set_cursor(&mut self, id: &str) {
        self.cursor = Some(id.to_string());
    }

    /// The autopilot's next skill: open remediation first, then due reviews, then the path
    /// frontier. Case/route ids are ignored here; the app resolves those from `cursor`.
    pub fn next_skill(&self, now: u64) -> Option<&'static str> {
        // The sprint skips reviews outside the goal: name what «Продолжить» will really give.
        if self.sprinting(now) {
            if self.goal_reached() {
                return self.route_left().first().copied().or_else(|| self.weakest_goal_at(self.sprint_until).first().copied());
            }
            let at_risk = self
                .weakest_goal_at(self.sprint_until)
                .into_iter()
                .find(|id| self.skills.get(*id).is_some_and(|p| p.level >= 2 && p.card.reps > 0 && p.card.retrievability(self.sprint_until) < 0.9));
            return at_risk.or_else(|| self.frontier());
        }
        if let Some(id) = self.remediation.iter().find(|id| SKILLS.iter().any(|s| s.id == id.as_str()))
            && let Some(s) = SKILLS.iter().find(|s| s.id == id.as_str())
        {
            return Some(s.id);
        }
        if let Some(id) = self.due(now).first() {
            return Some(*id);
        }
        self.frontier()
    }

    /// Extra practice (choosing a function, control checks, supply tasks) was added inside
    /// units learners may have finished; it never closes a unit or the skills after it.
    pub fn extra(id: &str) -> bool {
        ["choose.", "check.", "supply.", "orion."].iter().any(|p| id.starts_with(p))
    }

    fn unit_done(&self, unit: usize) -> bool {
        SKILLS.iter().filter(|s| s.unit == unit && !Self::extra(s.id)).all(|s| self.level(s.id) >= 2)
    }

    pub fn unit_open(&self, unit: usize) -> bool {
        UNIT_NEEDS.get(unit).is_none_or(|needs| needs.iter().all(|u| self.unit_done(*u)))
    }

    /// Within a unit, a skill opens when the previous one is at level 2 (or was ever passed twice).
    pub fn unlocked(&self, id: &str) -> bool {
        let Some(i) = SKILLS.iter().position(|s| s.id == id) else { return false };
        let Some(sk) = SKILLS.get(i) else { return false };
        if !self.unit_open(sk.unit) {
            return false;
        }
        // The previous core skill of the unit gates both the next core skill and any extra.
        match SKILLS.get(..i).and_then(|before| before.iter().rev().find(|p| !Self::extra(p.id))) {
            Some(prev) if prev.unit == sk.unit => self.level(prev.id) >= 2 || self.skills.get(prev.id).is_some_and(|p| p.successes >= 2),
            _ => true,
        }
    }

    pub fn case_open(&self, id: &str) -> bool {
        CASES.iter().find(|c| c.id == id).is_some_and(|c| c.requires.iter().all(|s| self.level(s) >= 2))
    }

    /// Prefer independent execution before extending practised skills to work level. Within
    /// open units the course heads for [`GOAL`] first, through whatever opens the next goal
    /// skill; otherwise it follows the path in order.
    pub fn frontier(&self) -> Option<&'static str> {
        self.frontier_excluding(&[])
    }

    /// The frontier without the skills skipped in the current lesson.
    pub fn frontier_excluding(&self, skip: &[String]) -> Option<&'static str> {
        self.toward_goal_excluding(skip).or_else(|| self.path_frontier_excluding(skip))
    }

    fn toward_goal(&self) -> Option<&'static str> {
        self.toward_goal_excluding(&[])
    }

    fn toward_goal_excluding(&self, skip: &[String]) -> Option<&'static str> {
        let skipped = |id: &str| skip.iter().any(|s| s == id);
        for goal in GOAL {
            let Some(at) = SKILLS.iter().position(|s| s.id == *goal) else { continue };
            let Some(sk) = SKILLS.get(at) else { continue };
            if !self.unit_open(sk.unit) {
                continue;
            }
            if self.unlocked(sk.id) {
                if self.level(sk.id) < 3 && !skipped(sk.id) {
                    return Some(sk.id);
                }
                continue;
            }
            // Locked: the first core skill of its unit that still holds it back.
            let blocker =
                SKILLS.get(..at).into_iter().flatten().filter(|s| s.unit == sk.unit && !Self::extra(s.id)).find(|s| {
                    self.unlocked(s.id) && self.level(s.id) < 2 && !self.skills.get(s.id).is_some_and(|p| p.successes >= 2) && !skipped(s.id)
                });
            if let Some(b) = blocker {
                return Some(b.id);
            }
        }
        None
    }

    fn path_frontier_excluding(&self, skip: &[String]) -> Option<&'static str> {
        let open = |s: &&crate::content::Skill| self.unlocked(s.id) && !skip.iter().any(|x| x == s.id);
        SKILLS.iter().filter(open).find(|s| self.level(s.id) < 3).or_else(|| SKILLS.iter().filter(open).find(|s| self.level(s.id) < 4)).map(|s| s.id)
    }

    pub fn due(&self, now: u64) -> Vec<&'static str> {
        let mut v: Vec<(&'static str, f64)> = SKILLS
            .iter()
            .filter_map(|s| {
                self.skills.get(s.id).filter(|p| p.level >= 2 && p.card.reps > 0 && p.card.due <= now).map(|p| (s.id, p.card.retrievability(now)))
            })
            .collect();
        v.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        v.into_iter().map(|(id, _)| id).collect()
    }

    /// A step is done once a task of that skill at that level (or harder) was solved.
    fn step_done(&self, id: &str, level: u8) -> bool {
        self.log.iter().any(|a| a.skill == id && a.passed && a.level >= level)
    }

    /// Route steps still to pass, in order (only open skills); one slot per step.
    fn route_left(&self) -> Vec<&'static str> {
        ROUTE.iter().filter(|(id, l)| self.unlocked(id) && !self.step_done(id, *l)).map(|(id, _)| *id).collect()
    }

    /// The level the route serves a skill at: its first unfinished step.
    fn route_level(&self, id: &str) -> Option<u8> {
        ROUTE.iter().find(|(s, l)| *s == id && !self.step_done(s, *l)).map(|(_, l)| *l)
    }

    fn in_route(id: &str) -> bool {
        ROUTE.iter().any(|(s, _)| *s == id)
    }

    /// Route levels apply while the interview route is what «Продолжить» walks.
    fn sprinting_route(&self, id: &str) -> bool {
        self.goal_reached() && self.route_level(id).is_some()
    }

    /// Inside the preparation sprint (before the dated goal).
    pub fn sprinting(&self, now: u64) -> bool {
        now < self.sprint_until
    }

    /// The interview goal is reached: every goal skill at «Сам».
    pub fn goal_reached(&self) -> bool {
        self.toward_goal().is_none() && GOAL.iter().all(|id| self.level(id) >= 3)
    }

    /// Goal skills ordered by how well FSRS expects them to be remembered at the deadline,
    /// weakest first; skills below «Как на работе» before those already there.
    fn weakest_goal_at(&self, at: u64) -> Vec<&'static str> {
        let mut v: Vec<(&'static str, u8, f64)> = GOAL
            .iter()
            .filter(|id| self.unlocked(id))
            .map(|id| (*id, self.level(id), self.skills.get(*id).map(|p| p.card.retrievability(at)).unwrap_or(0.0)))
            .collect();
        v.sort_by(|a, b| (a.1 >= 4).cmp(&(b.1 >= 4)).then(a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal)));
        v.into_iter().map(|(id, ..)| id).collect()
    }

    /// Sprint lessons. Before the goal: one review of the goal skill most at risk of being
    /// forgotten by the deadline, then only frontier work toward the goal. After it: every task
    /// is a goal skill, weakest-remembered first, at its own level (raising them to «Как на
    /// работе») — the morning warm-up and the practice for confidence.
    fn plan_sprint_lesson(&mut self) -> Lesson {
        let len = 8usize;
        let mut queue = VecDeque::new();
        let at = self.sprint_until;
        let route = self.route_left();
        if self.goal_reached() && !route.is_empty() {
            for id in route.into_iter().take(len) {
                queue.push_back(Slot::Focus(id.into()));
            }
        } else if self.goal_reached() {
            for id in self.weakest_goal_at(at).into_iter().cycle().take(len) {
                queue.push_back(Slot::Focus(id.into()));
            }
        } else {
            let at_risk = self
                .weakest_goal_at(at)
                .into_iter()
                .find(|id| self.skills.get(*id).is_some_and(|p| p.level >= 2 && p.card.reps > 0 && p.card.retrievability(at) < 0.9));
            if let Some(r) = at_risk {
                queue.push_back(Slot::Review(r.into()));
            }
            while queue.len() < len {
                queue.push_back(Slot::Frontier);
            }
        }
        Lesson { total: queue.len(), queue, ..Lesson::default() }
    }

    pub fn plan_lesson(&mut self, now: u64, focus: Option<&str>) -> Lesson {
        if focus.is_none() && self.sprinting(now) {
            return self.plan_sprint_lesson();
        }
        let len = 8usize;
        // A route node clicked on the map: that task first, then the rest of the route, no reviews.
        if let Some(f) = focus
            && Self::in_route(f)
        {
            let mut queue: VecDeque<Slot> = VecDeque::from([Slot::Focus(f.into())]);
            for id in self.route_left().into_iter().skip_while(|id| *id != f).skip(1).take(len - 1) {
                queue.push_back(Slot::Focus(id.into()));
            }
            return Lesson { total: queue.len(), queue, ..Lesson::default() };
        }
        let mut queue = VecDeque::new();
        let target = focus.or_else(|| self.frontier());
        let domain = |id: &str| {
            if id.starts_with("sql.") {
                1
            } else if id.starts_with("acc.") {
                2
            } else {
                0
            }
        };
        let remedy: Vec<String> = self.remediation.iter().filter(|s| target.is_none_or(|id| domain(id) == domain(s))).take(2).cloned().collect();
        self.remediation.retain(|s| !remedy.contains(s));
        for r in &remedy {
            queue.push_back(Slot::Retry(r.clone()));
        }
        let due = self.due(now);
        let reviews: Vec<&str> = due
            .iter()
            .copied()
            .filter(|d| !remedy.iter().any(|r| r == d) && Some(*d) != focus && target.is_none_or(|id| domain(id) == domain(d)))
            .take(2)
            .collect();
        // Do not interrupt an Excel lesson with an unrelated SQL/accounting drill.
        let mastered: Vec<&str> = SKILLS
            .iter()
            .filter(|s| {
                self.level(s.id) >= 3 && !reviews.contains(&s.id) && Some(s.id) != focus && target.is_none_or(|id| domain(id) == domain(s.id))
            })
            .map(|s| s.id)
            .collect();
        let mix = mastered.get((self.next_seed as usize).wrapping_mul(7) % mastered.len().max(1)).copied();
        if let Some(first) = reviews.first() {
            queue.push_back(Slot::Review((*first).into()));
        }
        let main = |q: &mut VecDeque<Slot>| match focus {
            Some(f) => q.push_back(Slot::Focus(f.into())),
            None => q.push_back(Slot::Frontier),
        };
        while queue.len() < len {
            let i = queue.len();
            if i == 3
                && let Some(m) = mix
            {
                queue.push_back(Slot::Mix(m.into()));
            } else if i == len - 2
                && let Some(r) = reviews.get(1)
            {
                queue.push_back(Slot::Review((*r).into()));
            } else {
                main(&mut queue);
            }
        }
        Lesson { total: queue.len(), queue, ..Lesson::default() }
    }

    pub fn seed(&mut self) -> u64 {
        self.next_seed = self.next_seed.wrapping_add(1);
        self.next_seed.wrapping_mul(0x2545_f491).wrapping_add(self.next_seed)
    }

    pub fn live_tier(&self, id: &str) -> u8 {
        self.live.get(id).map(|p| p.tier.clamp(1, crate::content::live::TIERS)).unwrap_or(1)
    }

    /// Resolves a lesson slot to a concrete exercise spec.
    pub fn spec_for(&mut self, slot: &Slot) -> Spec {
        let seed = self.seed();
        if let Slot::Live(id) = slot {
            return crate::content::live::spec(id, self.live_tier(id), seed, 1);
        }
        let id = match slot {
            Slot::Frontier => self.frontier().map(str::to_string).unwrap_or_else(|| self.weakest()),
            Slot::Review(s) | Slot::Mix(s) | Slot::Retry(s) | Slot::Focus(s) | Slot::TestOut(s) | Slot::Sprint(s) | Slot::Live(s) => s.clone(),
        };
        let level = match slot {
            Slot::TestOut(_) => self.level(&id).max(3),
            Slot::Sprint(_) => SPRINT_LEVEL,
            // The route builds on basics already at «Сам»: no copying stage.
            _ if self.sprinting_route(&id) => self.route_level(&id).unwrap_or(2),
            _ => self.level(&id).max(1),
        };
        // After a miss, exactly one task one step easier (a retry is that step already); once it
        // is solved the learner is back on level. A level drop after two misses is the step
        // itself, so the two never stack: one miss costs about five tasks, not ten.
        let pending_miss = self.skills.get(&id).is_some_and(|p| p.misses > 0);
        let level = if matches!(slot, Slot::Retry(_)) || (pending_miss && !matches!(slot, Slot::TestOut(_) | Slot::Sprint(_))) {
            level.saturating_sub(1).max(1)
        } else {
            level
        };
        Spec::new(&id, level, seed)
    }

    fn weakest(&self) -> String {
        SKILLS
            .iter()
            .filter(|s| self.unlocked(s.id))
            .min_by(|a, b| {
                let ra = self.skills.get(a.id).map(|p| p.card.retrievability(0)).unwrap_or(0.0);
                let rb = self.skills.get(b.id).map(|p| p.card.retrievability(0)).unwrap_or(0.0);
                ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.id.to_string())
            .unwrap_or_else(|| "start.enter".into())
    }

    fn add_xp(&mut self, xp: u32, now: u64, change: &mut Change) {
        if xp == 0 {
            return;
        }
        self.refresh_quests(now);
        let today = self.day(now);
        let before = self.today_xp(now);
        self.xp = self.xp.saturating_add(xp);
        *self.days.entry(today).or_default() += xp;
        if self.last_day != today {
            if self.last_day + 1 == today {
                self.streak = self.streak.saturating_add(1);
            } else if self.last_day > 0 && self.last_day + 2 == today && self.freezes > 0 && self.streak > 0 {
                // One missed day, covered by a freeze.
                self.freezes -= 1;
                self.frozen_days.push(today - 1);
                self.streak = self.streak.saturating_add(1);
                change.freeze_used = true;
            } else {
                if self.wager.is_some() {
                    self.wager = None;
                    change.wager_lost = true;
                }
                self.streak = 1;
            }
            self.last_day = today;
            change.streak_extended = true;
            if let Some(w) = self.wager.clone()
                && today >= w.start_day + u64::from(w.days)
            {
                let win = w.stake * 2;
                self.gems = self.gems.saturating_add(win);
                self.wager = None;
                change.wager_won = Some(win);
                self.unlock("wager_won", now, change);
            }
        }
        change.goal_reached = before < self.goal && before + xp >= self.goal;
        if change.goal_reached {
            self.gems = self.gems.saturating_add(10);
            change.gems += 10;
        }
        self.quest(QuestKind::Xp, xp, now, change);
        for (id, ok) in [
            ("streak_3", self.streak >= 3),
            ("streak_7", self.streak >= 7),
            ("streak_30", self.streak >= 30),
            ("xp_500", self.xp >= 500),
            ("xp_3000", self.xp >= 3000),
        ] {
            if ok {
                self.unlock(id, now, change);
            }
        }
    }

    pub fn unlock(&mut self, id: &str, now: u64, change: &mut Change) {
        if !self.achievements.contains_key(id) {
            self.achievements.insert(id.into(), now);
            self.gems = self.gems.saturating_add(20);
            change.gems += 20;
            change.achievements.push(id.into());
        }
    }

    /// Three quests a day, chosen from the day number so they are stable within the day.
    pub fn refresh_quests(&mut self, now: u64) {
        let today = self.day(now);
        if self.quest_day == today && !self.quests.is_empty() {
            return;
        }
        self.quest_day = today;
        let pool = [
            (QuestKind::Lightning, 3, 10),
            (QuestKind::Perfect, 1, 15),
            (QuestKind::Combo, 6, 10),
            (QuestKind::Lessons, 2, 10),
            (QuestKind::Sql, 3, 10),
            (QuestKind::Blitz, 1, 15),
            (QuestKind::Review, 3, 10),
        ];
        let mut picks = vec![(QuestKind::Xp, self.goal, 10)];
        let mut k = today.wrapping_mul(2_654_435_761) as usize;
        while picks.len() < 3 {
            let p = pool[k % pool.len()];
            if !picks.iter().any(|q| q.0 == p.0) {
                picks.push(p);
            }
            k = k / 7 + 3;
        }
        self.quests = picks.into_iter().map(|(kind, target, reward)| Quest { kind, target, progress: 0, reward, done: false }).collect();
    }

    /// Advances quests of `kind`; `amount` is added (or, for Combo, the best value is kept).
    pub fn quest(&mut self, kind: QuestKind, amount: u32, now: u64, change: &mut Change) {
        self.refresh_quests(now);
        let mut reward = 0;
        for q in self.quests.iter_mut().filter(|q| q.kind == kind && !q.done) {
            q.progress = if kind == QuestKind::Combo { q.progress.max(amount) } else { q.progress.saturating_add(amount) };
            if q.progress >= q.target {
                q.progress = q.target;
                q.done = true;
                reward += q.reward;
                change.quests_done.push(q.title());
            }
        }
        if reward > 0 {
            self.gems = self.gems.saturating_add(reward);
            change.gems += reward;
        }
    }

    /// Bets `stake` gems on keeping the streak for `days` more days; a win pays double.
    pub fn place_wager(&mut self, stake: u32, days: u32, now: u64) -> Result<(), String> {
        if self.wager.is_some() {
            return Err(ts("Ставка уже сделана").into());
        }
        if self.gems < stake {
            return Err(tr(&format!("Нужно {stake} 💎")));
        }
        self.gems -= stake;
        self.wager = Some(Wager { start_day: self.day(now), days, stake });
        Ok(())
    }

    pub const FREEZE_PRICE: u32 = 100;

    pub fn buy_freeze(&mut self) -> Result<(), String> {
        if self.freezes >= 2 {
            return Err(ts("Больше двух заморозок не хранится").into());
        }
        if self.gems < Self::FREEZE_PRICE {
            return Err(tr(&format!("Нужно {} 💎", Self::FREEZE_PRICE)));
        }
        self.gems -= Self::FREEZE_PRICE;
        self.freezes += 1;
        Ok(())
    }

    /// Lesson finished: gems, quests and badges.
    pub fn finish_lesson(&mut self, perfect: bool, best_combo: u32, now: u64) -> Change {
        let mut change = Change::default();
        self.lessons = self.lessons.saturating_add(1);
        let mut gems = 5;
        if perfect {
            self.perfect_lessons = self.perfect_lessons.saturating_add(1);
            gems += 10;
            self.quest(QuestKind::Perfect, 1, now, &mut change);
        }
        self.gems = self.gems.saturating_add(gems);
        change.gems += gems;
        self.best_combo = self.best_combo.max(best_combo);
        self.quest(QuestKind::Lessons, 1, now, &mut change);
        self.unlock("first_lesson", now, &mut change);
        for (id, ok) in [("perfect_1", self.perfect_lessons >= 1), ("perfect_10", self.perfect_lessons >= 10), ("combo_10", self.best_combo >= 10)] {
            if ok {
                self.unlock(id, now, &mut change);
            }
        }
        change
    }

    /// A finished blitz: XP by points, quest and badge.
    /// One solved task in the accounting program: base XP plus a speed bonus against par.
    pub fn record_ledger_task(&mut self, seconds: f32, goal_seconds: u32, now: u64) -> Change {
        let mut change = Change::default();
        let ratio = seconds / goal_seconds.max(1) as f32;
        change.speed = if ratio <= 0.4 {
            Speed::Lightning
        } else if ratio <= 0.7 {
            Speed::Fast
        } else {
            Speed::Normal
        };
        change.speed_bonus = match change.speed {
            Speed::Lightning => 10,
            Speed::Fast => 5,
            Speed::Normal => 0,
        };
        if change.speed == Speed::Lightning {
            self.lightning = self.lightning.saturating_add(1);
        }
        self.add_xp(10 + change.speed_bonus, now, &mut change);
        change
    }

    pub fn record_blitz(&mut self, result: crate::quiz::BlitzResult, missed: &[String], now: u64) -> Change {
        let mut change = Change::default();
        let xp = result.points / 2;
        self.add_xp(xp, now, &mut change);
        change.xp = xp;
        self.quest(QuestKind::Blitz, 1, now, &mut change);
        if result.total > 0 && result.correct * 10 >= result.total * 8 {
            self.unlock("blitz_80", now, &mut change);
        }
        for m in missed {
            if SKILLS.iter().any(|s| s.id == m) && !self.remediation.contains(m) && self.unlocked(m) {
                self.remediation.push(m.clone());
            }
        }
        self.remediation.truncate(6);
        self.blitz.push(result);
        if self.blitz.len() > 200 {
            self.blitz.drain(..self.blitz.len() - 200);
        }
        change
    }

    /// Skills good for a speed run: practised to level 2 or more.
    pub fn sprint_skills(&self) -> Vec<&'static str> {
        SKILLS.iter().filter(|s| self.level(s.id) >= 2).map(|s| s.id).collect()
    }

    /// Ten tasks against the clock: one skill, or a mix of practised skills.
    pub fn plan_sprint(&mut self, skill: Option<&str>) -> Lesson {
        let pool: Vec<String> = match skill {
            Some(s) => vec![s.to_string()],
            None => {
                let v = self.sprint_skills();
                if v.is_empty() { vec!["start.add".into(), "start.mul".into(), "fn.sum".into()] } else { v.iter().map(|s| s.to_string()).collect() }
            }
        };
        let mut queue = VecDeque::new();
        let mut k = self.seed() as usize;
        for i in 0..SPRINT_LEN {
            let id = pool.get((k + i * 7) % pool.len().max(1)).cloned().unwrap_or_else(|| "start.add".into());
            k = k.wrapping_mul(31).wrapping_add(17) % 10_007;
            queue.push_back(Slot::Sprint(id));
        }
        Lesson { total: queue.len(), queue, retries: 3, sprint: Some(skill.unwrap_or("mix").to_string()), ..Lesson::default() }
    }

    pub fn sprint_best(&self, kind: &str) -> Option<f32> {
        self.sprints.iter().filter(|r| r.kind == kind).map(|r| r.seconds).fold(None, |m, x| Some(m.map_or(x, |m: f32| m.min(x))))
    }

    /// Stores a finished sprint; returns (medal, new personal record, change).
    pub fn record_sprint(&mut self, result: SprintResult, now: u64) -> (Medal, bool, Change) {
        let mut change = Change::default();
        let best = self.sprint_best(&result.kind);
        let record = best.is_none_or(|b| result.seconds < b);
        let medal = Medal::of(result.seconds, result.par);
        let gems = match medal {
            Medal::Gold => 15,
            Medal::Silver => 10,
            Medal::Bronze => 5,
            Medal::None => 0,
        } + if record && best.is_some() { 10 } else { 0 };
        self.gems = self.gems.saturating_add(gems);
        change.gems += gems;
        if medal == Medal::Gold {
            self.unlock("sprint_gold", now, &mut change);
        }
        if record && best.is_some() {
            self.unlock("sprint_record", now, &mut change);
        }
        self.sprints.push(result);
        if self.sprints.len() > 500 {
            self.sprints.drain(..self.sprints.len() - 500);
        }
        (medal, record, change)
    }

    /// A lesson that checks every skill of a unit at level 3; clean passes skip ahead.
    pub fn plan_testout(&self, unit: usize) -> Lesson {
        let queue: VecDeque<Slot> =
            SKILLS.iter().filter(|s| s.unit == unit && self.level(s.id) < 3).take(8).map(|s| Slot::TestOut(s.id.into())).collect();
        Lesson { total: queue.len(), queue, retries: 3, ..Lesson::default() }
    }

    /// Records a drill answer: the first evaluation of a task decides the grade.
    /// `combo` is the number of correct answers in a row before this one.
    pub fn record(&mut self, task: &Task, passed: bool, seconds: f32, hints: u32, combo: u32, test_out: bool, now: u64) -> Change {
        if let Some(base) = crate::content::live::base(&task.spec.skill) {
            return self.record_live(base, task, passed, seconds, hints, combo, now);
        }
        let mut change = Change::default();
        let id = task.spec.skill.clone();
        let level = task.spec.level;
        let sprint = self.sprinting(now);
        let p = self.skills.entry(id.clone()).or_default();
        if p.level == 0 {
            // A route skill starts where it was served (level 2), not at copying.
            p.level = if Self::in_route(&id) { level.clamp(1, 2) } else { 1 };
        }
        p.attempts = p.attempts.saturating_add(1);
        p.seconds += seconds.max(0.0);
        p.hints = p.hints.saturating_add(hints);
        let ratio = seconds / task.goal_seconds.max(1.0);
        // A shown example is support even without an explicit hint click.
        let assisted = hints > 0 || task.example.is_some();
        self.evidence.entry(id.clone()).or_default().observe(crate::evidence::Observation {
            at: now,
            passed,
            assisted,
            seconds,
            target_seconds: task.goal_seconds,
            variant: task.spec.layout.rotate_left(17) ^ task.spec.data,
            independent: !assisted && level >= 3,
            // A variant probe tests the formula, not the learner's transfer.
            transfer: false,
        });
        let speed = if !passed || assisted || !seconds.is_finite() || seconds < 0.0 {
            Speed::Normal
        } else if ratio <= 0.4 {
            Speed::Lightning
        } else if ratio <= 0.7 {
            Speed::Fast
        } else {
            Speed::Normal
        };
        change.speed = speed;
        let grade = if !passed {
            Grade::Again
        } else if assisted {
            Grade::Hard
        } else if speed != Speed::Normal && level >= 2 {
            Grade::Easy
        } else {
            Grade::Good
        };
        // Only exercises at the skill's current level move the staircase.
        let on_level = level >= p.level;
        if passed {
            p.successes = p.successes.saturating_add(1);
            if seconds.is_finite() && seconds >= 0.0 && (p.best_seconds == 0.0 || seconds < p.best_seconds) {
                p.best_seconds = seconds;
            }
            p.misses = 0;
            if !assisted && test_out && p.level < 3 {
                // Skip-ahead check passed: the skill is known.
                p.level = 3;
                p.clean = 0;
                change.level_up = Some(3);
                change.fast_track = true;
            } else if level == 1 && on_level && hints == 0 {
                // Repeating examples opens supported practice, never independent mastery.
                p.clean = p.clean.saturating_add(1);
                if p.clean >= successes_needed(&id, 1, sprint) {
                    p.level = 2;
                    p.clean = 0;
                    change.level_up = Some(2);
                }
            } else if !assisted && on_level {
                // The lookup composition ladder counts each correct practice once; speed
                // never shortcuts the repetitions that make the syntax automatic.
                let massed = matches!(id.as_str(), "lookup.error_observe" | "lookup.iferror.wrap" | "lookup.iferror.combine");
                p.clean = p.clean.saturating_add(if !massed && speed != Speed::Normal { 2 } else { 1 });
                let need = successes_needed(&id, p.level, sprint);
                if p.level < 4 && p.clean >= need {
                    p.level += 1;
                    p.clean = 0;
                    change.level_up = Some(p.level);
                }
            } else {
                p.clean = 0;
            }
        } else {
            p.clean = 0;
            p.misses = p.misses.saturating_add(1);
            if p.misses >= 2 && p.level > 1 {
                p.level -= 1;
                p.misses = 0;
                change.level_down = true;
            }
        }
        // Level-1 copying is learning, not recall: reviews start once the skill is done unaided.
        let was_review = p.card.reps > 0;
        if level >= 2 || !passed || change.fast_track {
            p.card.review(grade, now);
        }
        let mut xp = 0;
        if passed {
            let base = (10 + 2 * u32::from(level)).saturating_sub(3 * hints).max(2);
            change.speed_bonus = match speed {
                Speed::Lightning => 10,
                Speed::Fast => 5,
                Speed::Normal => 0,
            };
            change.combo_bonus = combo.min(5) * 2;
            xp = base + change.speed_bonus + change.combo_bonus;
        }
        self.add_xp(xp, now, &mut change);
        change.xp = xp;
        if passed {
            if speed == Speed::Lightning {
                self.lightning = self.lightning.saturating_add(1);
                self.quest(QuestKind::Lightning, 1, now, &mut change);
            }
            self.quest(QuestKind::Combo, combo + 1, now, &mut change);
            self.best_combo = self.best_combo.max(combo + 1);
            if was_review {
                self.quest(QuestKind::Review, 1, now, &mut change);
            }
            if id.starts_with("sql.") {
                self.sql_solved = self.sql_solved.saturating_add(1);
                self.quest(QuestKind::Sql, 1, now, &mut change);
            }
        }
        for (b, ok) in [
            ("lightning_10", self.lightning >= 10),
            ("lightning_100", self.lightning >= 100),
            ("combo_10", self.best_combo >= 10),
            ("sql_10", self.sql_solved >= 10),
            ("sql_master", SKILLS.iter().filter(|s| s.id.starts_with("sql.")).all(|s| self.level(s.id) >= 3)),
        ] {
            if ok {
                self.unlock(b, now, &mut change);
            }
        }
        self.log.push(Attempt { skill: id, level, at: now, seconds, hints, passed, mode: task.mode });
        if self.log.len() > 5000 {
            self.log.drain(..self.log.len() - 5000);
        }
        change
    }

    /// Whether the live register of a skill may be offered now (not resting after two misses).
    pub fn live_ready(&self, skill: &str, now: u64) -> bool {
        self.live.get(skill).is_none_or(|p| p.rest_until <= now)
    }

    /// Interview goal: how many goal skills are at level 3 («Сам») or higher, out of how many,
    /// and the skill the course works on next to get there.
    pub fn goal_progress(&self) -> (usize, usize, Option<&'static str>) {
        let done = GOAL.iter().filter(|id| self.level(id) >= 3).count();
        (done, GOAL.len(), self.toward_goal())
    }

    /// The whole way to the interview goal: every goal skill to level 3, everything that opens
    /// it (earlier core skills of its unit, and of the units it needs) to level 2. Fixed for a
    /// given curriculum, so the percentage only moves when the learner does.
    fn goal_route(&self) -> Vec<(&'static str, u8)> {
        self.route_to(GOAL)
    }

    /// The route to these goal skills (see [`Self::goal_route`]).
    fn route_to(&self, goals: &[&str]) -> Vec<(&'static str, u8)> {
        let mut route: Vec<(&'static str, u8)> = Vec::new();
        let push = |id: &'static str, target: u8, route: &mut Vec<(&'static str, u8)>| match route.iter_mut().find(|(r, _)| *r == id) {
            Some((_, t)) => *t = (*t).max(target),
            None => route.push((id, target)),
        };
        let mut units: Vec<usize> = Vec::new();
        for goal in goals {
            let Some(at) = SKILLS.iter().position(|s| s.id == *goal) else { continue };
            let Some(sk) = SKILLS.get(at) else { continue };
            // Only the units a goal unit builds on must be finished, not the goal unit itself.
            let mut stack: Vec<usize> = UNIT_NEEDS.get(sk.unit).copied().unwrap_or(&[]).to_vec();
            while let Some(u) = stack.pop() {
                if units.contains(&u) {
                    continue;
                }
                units.push(u);
                stack.extend(UNIT_NEEDS.get(u).copied().unwrap_or(&[]).iter().copied());
            }
            for before in SKILLS.get(..at).into_iter().flatten().filter(|s| s.unit == sk.unit && !Self::extra(s.id)) {
                push(before.id, 2, &mut route);
            }
            push(sk.id, 3, &mut route);
        }
        // Units the goal units build on must be done (core skills at level 2).
        for s in SKILLS.iter().filter(|s| !Self::extra(s.id) && units.contains(&s.unit)) {
            push(s.id, 2, &mut route);
        }
        route
    }

    /// Clean passes still needed to lift a skill to `target` (the staircase thresholds).
    fn passes_to(&self, id: &str, target: u8, sprint: bool) -> u32 {
        let p = self.skills.get(id);
        let level = p.map(|p| p.level).unwrap_or(0).max(1);
        if self.level(id) >= target {
            return 0;
        }
        let clean = u32::from(p.map(|p| p.clean).unwrap_or(0));
        let total: u32 = (level..target).map(|l| u32::from(successes_needed(id, l, sprint))).sum();
        total.saturating_sub(clean).max(1)
    }

    /// Progress toward the interview goal: percent done, clean passes left, and an estimate in
    /// minutes from this learner's own pace (seconds per task and first-try success rate).
    pub fn goal_plan(&self, now: u64) -> GoalPlan {
        let sprint = self.sprinting(now);
        let route = self.goal_route();
        let from_zero: u32 = route.iter().map(|(id, t)| (1..*t).map(|l| u32::from(successes_needed(id, l, sprint))).sum::<u32>()).sum();
        // What is left leads only to goal skills not reached yet: steps toward a goal skill
        // already at «Сам» (sorting before a mastered pivot) are no longer on the way.
        let open: Vec<&str> = GOAL.iter().copied().filter(|g| self.level(g) < 3).collect();
        let left: u32 = self.route_to(&open).iter().map(|(id, t)| self.passes_to(id, *t, sprint)).sum();
        // Pace from real attempts; quick skips (under 8 s) are not attempts.
        let recent: Vec<&Attempt> = self.log.iter().rev().filter(|a| a.seconds >= 8.0).take(60).collect();
        let (secs, rate) = if recent.len() >= 10 {
            // Mean with long breaks capped at 3 minutes: close to the real active time per task.
            let secs = recent.iter().map(|a| f64::from(a.seconds.clamp(8.0, 180.0))).sum::<f64>() / recent.len() as f64;
            let rate = recent.iter().filter(|a| a.passed).count() as f64 / recent.len() as f64;
            (secs, rate)
        } else {
            (60.0, 0.7)
        };
        // Each miss costs about one more task on top of the redo (the easier step after a miss);
        // outside the sprint, reviews and mixing take roughly half of a lesson.
        let overhead = if sprint { 1.15 } else { 2.0 };
        let tasks = f64::from(left) * (1.0 + 2.0 * (1.0 - rate.clamp(0.3, 1.0))) * overhead;
        GoalPlan {
            percent: if from_zero == 0 { 100 } else { (100 - (u64::from(left) * 100 / u64::from(from_zero)).min(100)) as u8 },
            passes_left: left,
            tasks_left: tasks.ceil() as u32,
            minutes_left: (tasks * secs / 60.0).ceil() as u32,
        }
    }

    /// Live data moves its own dirt tier only: a messy register is transfer evidence for the
    /// skill, but it never pushes the clean-drill staircase or its review schedule.
    #[allow(clippy::too_many_arguments)]
    fn record_live(&mut self, base: &str, task: &Task, passed: bool, seconds: f32, hints: u32, combo: u32, now: u64) -> Change {
        let mut change = Change::default();
        let assisted = hints > 0;
        let p = self.live.entry(base.to_string()).or_default();
        p.tier = p.tier.clamp(1, crate::content::live::TIERS);
        p.attempts = p.attempts.saturating_add(1);
        let on_tier = task.spec.level >= p.tier;
        if passed {
            p.passes = p.passes.saturating_add(1);
            p.misses = 0;
            if !assisted && on_tier {
                p.clean = p.clean.saturating_add(1);
                if p.clean >= LIVE_STEP && p.tier < crate::content::live::TIERS {
                    p.tier += 1;
                    p.clean = 0;
                    change.live_up = Some(p.tier);
                }
            }
        } else {
            p.clean = 0;
            p.misses = p.misses.saturating_add(1);
            if p.misses >= 2 {
                // Looping on the same kind of register teaches little: explain it, rest it.
                p.rest_until = now.saturating_add(LIVE_REST);
                change.live_rest = true;
                if p.tier > 1 {
                    p.tier -= 1;
                    change.level_down = true;
                }
                p.misses = 0;
            }
        }
        // A due review answered on live data counts as that review.
        if passed
            && !assisted
            && let Some(sp) = self.skills.get_mut(base)
            && sp.card.reps > 0
            && sp.card.due <= now
        {
            sp.card.review(Grade::Good, now);
        }
        self.evidence.entry(base.to_string()).or_default().observe(crate::evidence::Observation {
            at: now,
            passed,
            assisted,
            seconds,
            target_seconds: task.goal_seconds,
            variant: task.spec.layout.rotate_left(17) ^ task.spec.data,
            independent: !assisted,
            // An unfamiliar hand-typed register is what transfer means here.
            transfer: true,
        });
        let mut xp = 0;
        if passed {
            change.combo_bonus = combo.min(5) * 2;
            xp = (14 + 4 * u32::from(task.spec.level)).saturating_sub(3 * hints).max(3) + change.combo_bonus;
        }
        self.add_xp(xp, now, &mut change);
        change.xp = xp;
        if passed {
            self.quest(QuestKind::Combo, combo + 1, now, &mut change);
            self.best_combo = self.best_combo.max(combo + 1);
        }
        self.log.push(Attempt { skill: task.spec.skill.clone(), level: task.spec.level, at: now, seconds, hints, passed, mode: task.mode });
        if self.log.len() > 5000 {
            self.log.drain(..self.log.len() - 5000);
        }
        change
    }

    /// Records a submitted case or interview; missed criteria send their skills back.
    pub fn record_case(&mut self, task: &Task, eval: &Evaluation, seconds: f32, interview: Option<u32>, now: u64) -> Change {
        let mut change = Change::default();
        let c = self.cases.entry(task.spec.skill.clone()).or_default();
        c.runs = c.runs.saturating_add(1);
        c.last = eval.score;
        c.best = c.best.max(eval.score);
        let mut missed: Vec<String> = Vec::new();
        for o in &eval.outcomes {
            if !o.passed && !missed.contains(&o.skill) {
                missed.push(o.skill.clone());
            }
        }
        let mut reviewed = Vec::new();
        for o in &eval.outcomes {
            if let Some(p) = self.skills.get_mut(&o.skill)
                && p.card.reps > 0
                && !missed.contains(&o.skill)
                && !reviewed.contains(&o.skill)
            {
                p.card.review(Grade::Good, now);
                reviewed.push(o.skill.clone());
            }
        }
        for m in &missed {
            if let Some(p) = self.skills.get_mut(m) {
                p.card.review(Grade::Again, now);
            }
            if !self.remediation.contains(m) {
                self.remediation.push(m.clone());
            }
        }
        self.remediation.truncate(6);
        let xp = (eval.score * 60.0).round() as u32 + if interview.is_some() { 20 } else { 0 };
        self.add_xp(xp, now, &mut change);
        change.xp = xp;
        if interview.is_some() && eval.score >= 0.8 {
            self.unlock("interview_80", now, &mut change);
        }
        if let Some(minutes) = interview {
            self.interviews.push(InterviewResult { case: task.spec.skill.clone(), at: now, score: eval.score, seconds, minutes, missed });
        }
        self.log.push(Attempt {
            skill: task.spec.skill.clone(),
            level: task.spec.level,
            at: now,
            seconds,
            hints: 0,
            passed: eval.passed(),
            mode: task.mode,
        });
        change
    }

    /// Mastery 0..1 over the whole path, for the progress ring.
    pub fn mastery(&self) -> f32 {
        let total = (SKILLS.len() * 4) as f32;
        SKILLS.iter().map(|s| f32::from(self.level(s.id).saturating_sub(1).min(3) + u8::from(self.level(s.id) >= 4))).sum::<f32>() / total
    }
}
