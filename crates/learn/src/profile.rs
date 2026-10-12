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
/// The skill check: the same eight skills every time, on new data, at level 3 (the task names
/// the business question, not the tool), with no hints. Taken before training, after it and a
/// week later, it shows whether the skill holds.
pub const CHECKPOINT_SKILLS: [&str; 8] = ["start.vat", "refs.abs", "fn.sum", "logic.if", "cond.sumif", "lookup.xlookup", "text.number", "date.diff"];
pub const CHECKPOINT_LEVEL: u8 = 3;
/// Battery skills at level 3+ that open the "after" check.
pub const CHECKPOINT_TRAINED: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Before,
    After,
    Later,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Before => "До обучения",
            Phase::After => "После обучения",
            Phase::Later => "Через неделю",
        }
    }
}

/// A task sent out as an .xlsx file and not yet solved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingTransfer {
    pub spec: Spec,
    pub path: String,
    pub at: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransferResult {
    pub case: String,
    pub at: u64,
    pub score: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub phase: Phase,
    pub at: u64,
    pub solved: u32,
    pub total: u32,
    pub seconds: f32,
}
/// A miss in a sprint adds this many seconds to the time.
pub const SPRINT_PENALTY: f32 = 10.0;
/// Clean successes needed at levels 1, 2, 3 before the next level.
const NEED: [u8; 3] = [2, 2, 3];
/// Units each unit builds on (it opens when those are all at level 2+).
const UNIT_NEEDS: [&[usize]; 12] = [&[], &[0], &[1], &[2], &[3], &[4], &[2], &[2], &[2], &[1], &[], &[]];

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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ActionProgress {
    pub attempts: u32,
    pub successes: u32,
}

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
    /// First encounters persist across exercises and restarts.
    pub seen_errors: Vec<String>,
    /// Error literacy tasks waiting for the next ordinary lesson slot.
    pub pending_errors: Vec<String>,
    pub xp: u32,
    /// Local day number → XP earned that day.
    pub days: BTreeMap<u64, u32>,
    pub streak: u32,
    pub last_day: u64,
    pub goal: u32,
    /// Hours east of UTC for day boundaries (Moscow by default).
    pub utc_offset: i64,
    pub skills: BTreeMap<String, SkillProgress>,
    /// First-submission evidence for individual actions, independent of whole-task success.
    pub actions: BTreeMap<String, ActionProgress>,
    pub cases: BTreeMap<String, CaseProgress>,
    /// Skills a failed case or interview sent back for practice.
    pub remediation: Vec<String>,
    pub log: Vec<Attempt>,
    pub interviews: Vec<InterviewResult>,
    pub next_seed: u64,
    pub sound: bool,
    pub dark: bool,
    /// Preferred spreadsheet magnification, reused for new exercises.
    pub sheet_zoom: u16,
    /// Preferred size of the whole interface.
    pub interface_scale: u16,
    /// English interface instead of Russian.
    #[serde(default)]
    pub english: bool,
    /// "Which method" questions: answered right on the first pick / asked.
    #[serde(default)]
    pub checkpoints: Vec<Checkpoint>,
    #[serde(default)]
    pub pending_transfer: Option<PendingTransfer>,
    #[serde(default)]
    pub transfers: Vec<TransferResult>,
    #[serde(default)]
    pub method_right: u32,
    #[serde(default)]
    pub method_total: u32,
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

impl Default for Profile {
    fn default() -> Self {
        Self {
            seen_errors: Vec::new(),
            pending_errors: Vec::new(),
            xp: 0,
            days: BTreeMap::new(),
            streak: 0,
            last_day: 0,
            goal: 50,
            utc_offset: 3,
            skills: BTreeMap::new(),
            actions: BTreeMap::new(),
            cases: BTreeMap::new(),
            remediation: Vec::new(),
            log: Vec::new(),
            interviews: Vec::new(),
            next_seed: 1,
            sound: true,
            dark: false,
            sheet_zoom: 100,
            interface_scale: 100,
            english: false,
            checkpoints: Vec::new(),
            pending_transfer: None,
            transfers: Vec::new(),
            method_right: 0,
            method_total: 0,
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
    /// The skill jumped ahead because it was clearly already known.
    pub fast_track: bool,
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
    /// Skill check task: fixed level, no hints, no retries, no effect on the staircase.
    Check(String),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Lesson {
    /// The entire skip-ahead assessment, including its final task.
    #[serde(default)]
    pub test_out: bool,
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
    /// Skills actually submitted in this lesson; preserved across restart.
    #[serde(default)]
    pub practised: Vec<String>,
    /// Speed run: "mix" or a skill id. Passes advance at once; the clock is what counts.
    #[serde(default)]
    pub sprint: Option<String>,
    /// Sum of the tasks' target times, for sprint medals.
    #[serde(default)]
    pub goal_total: f32,
    #[serde(default)]
    pub misses: u32,
    /// A skill check rather than a lesson.
    #[serde(default)]
    pub checkpoint: Option<Phase>,
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

    fn unit_done(&self, unit: usize) -> bool {
        SKILLS.iter().filter(|s| s.unit == unit).all(|s| self.level(s.id) >= 2)
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
        match i.checked_sub(1).and_then(|p| SKILLS.get(p)) {
            Some(prev) if prev.unit == sk.unit => self.level(prev.id) >= 2 || self.skills.get(prev.id).is_some_and(|p| p.successes >= 2),
            _ => true,
        }
    }

    pub fn case_open(&self, id: &str) -> bool {
        CASES.iter().find(|c| c.id == id).is_some_and(|c| c.requires.iter().all(|s| self.level(s) >= 2))
    }

    /// The next skill to climb: first open skill below level 3 in path order.
    pub fn frontier(&self) -> Option<&'static str> {
        SKILLS.iter().find(|s| self.unlocked(s.id) && self.level(s.id) < 3).map(|s| s.id)
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

    pub fn plan_lesson(&mut self, now: u64, focus: Option<&str>) -> Lesson {
        let len = 8usize;
        let mut queue = VecDeque::new();
        // Error repair skills use the same FSRS cards, with separate discovery so they
        // need not appear as a new curriculum unit or unlock prerequisites.
        let mut error_reviews: Vec<_> =
            self.skills.iter().filter(|(id, progress)| id.starts_with("errors.") && progress.card.reps > 0 && progress.card.due <= now).collect();
        error_reviews.sort_by_key(|(_, progress)| progress.card.due);
        for (id, _) in error_reviews.into_iter().take(2) {
            queue.push_back(Slot::Review(id.clone()));
        }
        let remedy: Vec<String> = self.remediation.drain(..self.remediation.len().min(2)).collect();
        for r in &remedy {
            let target = crate::diagnostics::prerequisites(r)
                .iter()
                .copied()
                .find(|id| {
                    let failures: Vec<_> = self.log.iter().rev().filter(|a| a.skill == *id).take(2).collect();
                    self.skills.get(*id).is_some_and(|p| p.misses >= 2) || failures.len() == 2 && failures.iter().all(|a| !a.passed)
                })
                .unwrap_or(r);
            queue.push_back(Slot::Retry(target.into()));
            if target != r && !self.remediation.contains(r) {
                self.remediation.push(r.clone());
            }
        }
        let due = self.due(now);
        let reviews: Vec<&str> = due.iter().copied().filter(|d| !remedy.iter().any(|r| r == d) && Some(*d) != focus).take(2).collect();
        let mastered: Vec<&str> =
            SKILLS.iter().filter(|s| self.level(s.id) >= 3 && !reviews.contains(&s.id) && Some(s.id) != focus).map(|s| s.id).collect();
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

    /// Resolves a lesson slot to a concrete exercise spec.
    pub fn spec_for(&mut self, slot: &Slot) -> Spec {
        let seed = self.seed();
        let id = match slot {
            Slot::Frontier => self.frontier().map(str::to_string).unwrap_or_else(|| self.weakest()),
            Slot::Review(s) | Slot::Mix(s) | Slot::Retry(s) | Slot::Focus(s) | Slot::TestOut(s) | Slot::Sprint(s) | Slot::Check(s) => s.clone(),
        };
        let level = match slot {
            Slot::TestOut(_) => self.level(&id).max(3),
            Slot::Sprint(_) => SPRINT_LEVEL,
            Slot::Check(_) => CHECKPOINT_LEVEL,
            _ => self.level(&id).max(1),
        };
        // A retry steps down one level so the second attempt is likely to succeed.
        let level = if matches!(slot, Slot::Retry(_)) { level.saturating_sub(1).max(1) } else { level };
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
    /// A graded file from real Excel. Misses go back to practice, like a case; a pass closes the task.
    pub fn record_transfer(&mut self, task: &crate::Task, eval: &crate::Evaluation, now: u64) -> Change {
        let mut change = Change::default();
        for o in eval.outcomes.iter().filter(|o| !o.passed) {
            if SKILLS.iter().any(|s| s.id == o.skill) && !self.remediation.contains(&o.skill) {
                self.remediation.push(o.skill.clone());
            }
        }
        self.remediation.truncate(6);
        let xp = (eval.score * 40.0).round() as u32;
        self.add_xp(xp, now, &mut change);
        change.xp = xp;
        self.transfers.push(TransferResult { case: task.spec.skill.clone(), at: now, score: eval.score });
        if eval.passed() {
            self.pending_transfer = None;
        }
        change
    }

    /// Battery skills already at level 3 or higher.
    pub fn checkpoint_trained(&self) -> usize {
        CHECKPOINT_SKILLS.iter().filter(|s| self.level(s) >= 3).count()
    }

    /// Which skill check is due, or why none is open yet.
    pub fn next_checkpoint(&self, now: u64) -> Result<Phase, String> {
        let last_after = self.checkpoints.iter().rev().find(|c| c.phase == Phase::After);
        let last = self.checkpoints.last();
        match last.map(|c| c.phase) {
            None => Ok(Phase::Before),
            Some(Phase::Before) => {
                let n = self.checkpoint_trained();
                if n >= CHECKPOINT_TRAINED {
                    Ok(Phase::After)
                } else {
                    Err(gridcraft_tr::t(&format!(
                        "Откроется, когда {CHECKPOINT_TRAINED} из {} навыков среза дойдут до уровня «Сам» (сейчас {n}).",
                        CHECKPOINT_SKILLS.len()
                    )))
                }
            }
            Some(Phase::After) => {
                let at = last_after.map(|c| c.at).unwrap_or(now);
                let ready = at + 7 * DAY;
                if now >= ready {
                    Ok(Phase::Later)
                } else {
                    let days = (ready - now).div_ceil(DAY);
                    Err(gridcraft_tr::t(&format!("Отложенный срез — через неделю после итогового, осталось {days} дн.")))
                }
            }
            // A new round: train more, then measure again.
            Some(Phase::Later) => Ok(Phase::After),
        }
    }

    pub fn plan_checkpoint(&mut self, phase: Phase) -> Lesson {
        let queue: VecDeque<Slot> = CHECKPOINT_SKILLS.iter().map(|s| Slot::Check((*s).to_string())).collect();
        Lesson { total: queue.len(), queue, retries: 3, checkpoint: Some(phase), ..Lesson::default() }
    }

    /// Stores a finished skill check. Effort counts for the day, not the score.
    pub fn record_checkpoint(&mut self, c: Checkpoint, now: u64) -> Change {
        let mut change = Change::default();
        self.add_xp(20, now, &mut change);
        change.xp = 20;
        self.checkpoints.push(c);
        change
    }

    /// The first pick on a "which method" question. A wrong tool sends its skill back for practice.
    pub fn record_choice(&mut self, skill: &str, ok: bool) {
        self.method_total += 1;
        if ok {
            self.method_right += 1;
        } else if SKILLS.iter().any(|s| s.id == skill) && !self.remediation.iter().any(|m| m == skill) {
            self.remediation.push(skill.into());
            self.remediation.truncate(6);
        }
    }

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

    /// Speed practice requires three recent unaided first-attempt successes.
    pub fn sprint_ready(&self, id: &str) -> bool {
        let recent: Vec<_> = self.log.iter().rev().filter(|a| a.skill == id && a.mode == Mode::Drill).take(3).collect();
        self.level(id) >= 2 && recent.len() == 3 && recent.iter().all(|a| a.passed && a.hints == 0 && a.level >= 2)
    }

    /// Skills with demonstrated accuracy before timed practice.
    pub fn sprint_skills(&self) -> Vec<&'static str> {
        SKILLS.iter().filter(|s| self.sprint_ready(s.id)).map(|s| s.id).collect()
    }

    /// Ten tasks against the clock: one skill, or a mix of practised skills.
    pub fn plan_sprint(&mut self, skill: Option<&str>) -> Lesson {
        let pool: Vec<String> = match skill {
            Some(s) => {
                if self.sprint_ready(s) {
                    vec![s.to_string()]
                } else {
                    Vec::new()
                }
            }
            None => {
                let v = self.sprint_skills();
                v.iter().map(|s| s.to_string()).collect()
            }
        };
        let mut queue = VecDeque::new();
        if pool.is_empty() {
            return Lesson { sprint: Some(skill.unwrap_or("mix").to_string()), ..Lesson::default() };
        }
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
        Lesson { total: queue.len(), queue, retries: 3, test_out: true, ..Lesson::default() }
    }

    /// Records a drill answer: the first evaluation of a task decides the grade.
    /// `combo` is the number of correct answers in a row before this one.
    pub fn record(&mut self, task: &Task, passed: bool, seconds: f32, hints: u32, combo: u32, test_out: bool, now: u64) -> Change {
        let mut change = Change::default();
        let id = task.spec.skill.clone();
        let level = task.spec.level;
        let p = self.skills.entry(id.clone()).or_default();
        if p.level == 0 {
            p.level = 1;
        }
        p.attempts = p.attempts.saturating_add(1);
        p.seconds += seconds.max(0.0);
        p.hints = p.hints.saturating_add(hints);
        let ratio = seconds / task.goal_seconds.max(1.0);
        let speed = if !passed || hints > 0 {
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
        } else if hints > 0 {
            Grade::Hard
        } else {
            Grade::Good
        };
        // Only exercises at the skill's current level move the staircase.
        let on_level = level >= p.level;
        if passed {
            p.successes = p.successes.saturating_add(1);
            if p.best_seconds == 0.0 || seconds < p.best_seconds {
                p.best_seconds = seconds;
            }
            p.misses = 0;
            if hints == 0 && test_out && p.level < 3 {
                // Skip-ahead check passed: the skill is known.
                p.level = 3;
                p.clean = 0;
                change.level_up = Some(3);
                change.fast_track = true;
            } else if hints == 0 && on_level {
                // Accuracy advances mastery; response speed does not substitute for recall.
                p.clean = p.clean.saturating_add(1);
                let need = NEED.get(usize::from(p.level.saturating_sub(1))).copied().unwrap_or(u8::MAX);
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
        let mut credited = Vec::new();
        for o in &eval.outcomes {
            if credited.contains(&o.skill) {
                continue;
            }
            credited.push(o.skill.clone());
            if let Some(p) = self.skills.get_mut(&o.skill)
                && p.card.reps > 0
                && !missed.contains(&o.skill)
            {
                p.card.review(Grade::Good, now);
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
