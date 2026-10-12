//! Local-first learning runtime over GridCraft: seeded exercises, deterministic grading,
//! a per-skill difficulty staircase and FSRS reviews. Numbers are computed by GridCraft,
//! never by an LLM.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented, clippy::unreachable)]

pub mod check;
pub mod content;
pub mod fsrs;
pub mod profile;
pub mod quiz;
pub mod rng;
pub mod sheet;
pub mod sql;
#[cfg(test)]
mod tests;

pub use check::{Check, Evaluation, Kind, Outcome, evaluate};
pub use profile::{Attempt, Lesson, Profile, Slot};

use gridcraft_engine::{DocState, Session, model::Workbook};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum LearnError {
    #[error(transparent)]
    Engine(#[from] gridcraft_engine::EngineError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
}
pub type Result<T> = std::result::Result<T, LearnError>;

/// Everything needed to regenerate an exercise exactly.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spec {
    pub skill: String,
    /// 1 = repeat after me, 2 = tool named, 3 = business question, 4 = work-like layout.
    pub level: u8,
    pub layout: u64,
    pub data: u64,
}

impl Spec {
    pub fn new(skill: &str, level: u8, seed: u64) -> Self {
        Self { skill: skill.into(), level: level.clamp(1, 4), layout: seed, data: seed.wrapping_mul(0x9e37_79b9).wrapping_add(17) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Drill,
    Case,
    Interview,
}

/// A message from a manager that frames an open case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Brief {
    pub from: String,
    pub subject: String,
    pub body: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub spec: Spec,
    pub mode: Mode,
    pub title: String,
    pub prompt: String,
    /// The exact formula to type, shown only at level 1.
    pub example: Option<String>,
    pub hints: Vec<String>,
    pub focus: String,
    pub setup: sheet::Setup,
    pub checks: Vec<Check>,
    pub goal_seconds: f32,
    /// Sorting/filtering/deduplication moves source rows; skip the "source intact" rule.
    pub reorders: bool,
    pub brief: Option<Brief>,
    /// SQL drills: the database and reference query (the sheet only shows the tables).
    #[serde(default)]
    pub sql: Option<sql::SqlTask>,
    /// Questions before the work: which tool fits and why.
    #[serde(default)]
    pub choices: Vec<Choice>,
}

/// A "which method and why" question asked before a task.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub question: String,
    pub options: Vec<ChoiceOption>,
    /// The skill to practise when the pick is wrong (empty for method questions).
    pub skill: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChoiceOption {
    pub text: String,
    pub ok: bool,
    /// Shown after the pick.
    pub why: String,
}

impl Task {
    pub fn generate(spec: &Spec) -> Result<Self> {
        content::build(spec)
    }
    pub fn session(&self) -> Result<Session> {
        self.setup.session(&self.focus)
    }
    /// Cells the learner is expected to fill; auto-check waits until they are all non-empty.
    pub fn answer_cells(&self) -> Vec<String> {
        self.checks
            .iter()
            .filter_map(|c| match &c.kind {
                Kind::Formula { cell, .. } | Kind::Value { cell, .. } => Some(cell.clone()),
                _ => None,
            })
            .collect()
    }
    pub fn keys(&self) -> Vec<String> {
        self.checks.iter().filter_map(|c| if let Kind::Keys { combo } = &c.kind { Some(combo.clone()) } else { None }).collect()
    }
    pub fn skill_name(&self) -> String {
        content::skill(&self.spec.skill)
            .map(|s| gridcraft_tr::ts(s.name).to_string())
            .or_else(|| content::cases::case(&self.spec.skill).map(|c| gridcraft_tr::ts(c.title).to_string()))
            .unwrap_or_else(|| self.title.clone())
    }
}

/// What is on screen when the app closes, so the next launch continues mid-exercise.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Current {
    pub task: Task,
    pub workbook: Workbook,
    pub lesson: Option<Lesson>,
    pub interview: Option<Interview>,
    pub elapsed: f32,
    pub hints: u32,
    #[serde(default)]
    pub keys: Vec<String>,
    /// The SQL query being written.
    #[serde(default)]
    pub query: String,
}

impl Current {
    pub fn session(&self) -> Session {
        let mut s = Session::new();
        s.add_document(DocState::new(self.workbook.clone(), None, "Учебная книга".into()));
        s
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interview {
    pub case: String,
    pub minutes: u32,
    pub started: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Save {
    pub version: u32,
    pub profile: Profile,
    pub current: Option<Current>,
}

pub const SAVE_VERSION: u32 = 2;

impl Save {
    pub fn encode(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 32 * 1024 * 1024 {
            return Err(LearnError::Invalid("Файл прогресса больше 32 МиБ".into()));
        }
        let s: Self = serde_json::from_slice(bytes)?;
        if s.version != SAVE_VERSION {
            return Err(LearnError::Invalid("Неизвестная версия файла прогресса; файл сохранён без изменений".into()));
        }
        if let Some(c) = &s.current
            && (c.workbook.sheets.len() != 1 || c.workbook.sheets.first().is_none_or(|sh| sh.cells.len() > 50_000) || !c.elapsed.is_finite())
        {
            return Err(LearnError::Invalid("Повреждённое упражнение в файле прогресса".into()));
        }
        Ok(s)
    }
}
