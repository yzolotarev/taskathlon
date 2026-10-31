//! Deterministic, conservative skill evidence, separate from FSRS scheduling.
//!
//! `Profile` persists this evidence per skill, records graded drill attempts and
//! uses `Reduce` recommendations when selecting ordinary practice difficulty.
//! Other recommendations are available for scenario/sport selection; they must
//! not be confused with a verified professional-readiness score.
//! Ordinary drills must not claim transfer or independence unless actually
//! tested. Delayed recall is recorded explicitly, not inferred from elapsed time.
//! Recommendations are practice decisions, not mastery probabilities or hiring
//! assessments. Callers supply elapsed seconds and timestamps; no native clock,
//! UI, or FSRS state is used here.

use serde::{Deserialize, Serialize};

const WINDOW: usize = 6;
const MIN_CLEAN: usize = 4;

/// One graded attempt. `independent` means an explicit independent task, not
/// merely the absence of a hint. `variant` identifies layout/data variation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub at: u64,
    pub passed: bool,
    pub assisted: bool,
    pub seconds: f32,
    pub target_seconds: f32,
    pub variant: u64,
    pub independent: bool,
    pub transfer: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrectnessEvidence {
    pub passed: u64,
    pub failed: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistanceEvidence {
    pub assisted: u64,
    pub unassisted: u64,
}

/// Invalid, negative, or non-finite durations never contribute speed evidence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurationEvidence {
    pub valid: u64,
    pub within_target: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecallEvidence {
    pub passed: u64,
    pub failed: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SkillEvidence {
    pub correctness: CorrectnessEvidence,
    pub assistance: AssistanceEvidence,
    pub duration: DurationEvidence,
    /// Distinct variants retained in the recent window, not a lifetime set.
    pub variants: Vec<u64>,
    pub transfer: CorrectnessEvidence,
    pub delayed_recall: RecallEvidence,
    recent: Vec<Observation>,
}

/// First scale: how much instructional support to provide.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Support {
    Guided,
    Scaffolded,
    Independent,
}

/// Second scale: the next practice demand. Reduce means reduce task demand,
/// not remove instructional support; Sprint is optional speed practice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Practice {
    Reduce,
    Repeat,
    Vary,
    Independent,
    Sprint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recommendation {
    pub support: Support,
    pub practice: Practice,
}

fn valid_duration(o: &Observation) -> bool {
    o.seconds.is_finite() && o.seconds >= 0.0 && o.target_seconds.is_finite() && o.target_seconds > 0.0
}

impl SkillEvidence {
    /// Records in call order; `at` is metadata, not an implicit recall signal.
    pub fn observe(&mut self, observation: Observation) {
        if observation.passed {
            self.correctness.passed = self.correctness.passed.saturating_add(1);
        } else {
            self.correctness.failed = self.correctness.failed.saturating_add(1);
        }
        if observation.assisted {
            self.assistance.assisted = self.assistance.assisted.saturating_add(1);
        } else {
            self.assistance.unassisted = self.assistance.unassisted.saturating_add(1);
        }
        if valid_duration(&observation) {
            self.duration.valid = self.duration.valid.saturating_add(1);
            if observation.seconds <= observation.target_seconds {
                self.duration.within_target = self.duration.within_target.saturating_add(1);
            }
        }
        if observation.transfer {
            if observation.passed {
                self.transfer.passed = self.transfer.passed.saturating_add(1);
            } else {
                self.transfer.failed = self.transfer.failed.saturating_add(1);
            }
        }
        self.recent.push(observation);
        if self.recent.len() > WINDOW {
            self.recent.drain(..self.recent.len() - WINDOW);
        }
        self.variants = self.recent.iter().map(|o| o.variant).collect();
        self.variants.sort_unstable();
        self.variants.dedup();
    }

    /// Only call after an actual delayed, unassisted recall assessment.
    /// FSRS review grades and scheduling remain the caller's separate concern.
    pub fn record_delayed_recall(&mut self, passed: bool) {
        if passed {
            self.delayed_recall.passed = self.delayed_recall.passed.saturating_add(1);
        } else {
            self.delayed_recall.failed = self.delayed_recall.failed.saturating_add(1);
        }
    }

    pub fn support(&self) -> Support {
        self.suggest().support
    }

    /// Uses a bounded window, never a single fast success. Any error retained
    /// in that window restores guidance; assistance blocks independent work.
    pub fn suggest(&self) -> Recommendation {
        let recent = &self.recent[self.recent.len().saturating_sub(WINDOW)..];
        let (support, practice) = if recent.iter().any(|o| !o.passed) {
            (Support::Guided, Practice::Reduce)
        } else if recent.len() < MIN_CLEAN || recent.iter().any(|o| o.assisted) {
            (Support::Guided, Practice::Repeat)
        } else {
            let varied = recent.iter().any(|o| o.variant != recent[0].variant);
            if !varied {
                (Support::Scaffolded, Practice::Vary)
            } else if !recent.iter().all(|o| o.independent) {
                (Support::Scaffolded, Practice::Independent)
            } else if recent.iter().all(|o| valid_duration(o) && o.seconds <= o.target_seconds) {
                (Support::Independent, Practice::Sprint)
            } else {
                (Support::Independent, Practice::Vary)
            }
        };
        Recommendation { support, practice }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean(variant: u64) -> Observation {
        Observation { at: variant, passed: true, assisted: false, seconds: 2.0, target_seconds: 10.0, variant, independent: true, transfer: false }
    }

    #[test]
    fn shown_sample_does_not_establish_independence() {
        let mut evidence = SkillEvidence::default();
        for variant in 0..6 {
            evidence.observe(Observation { assisted: true, ..clean(variant) });
        }
        assert_eq!(evidence.support(), Support::Guided);
        assert_eq!(evidence.suggest().practice, Practice::Repeat);
    }

    #[test]
    fn errors_restore_support_until_window_is_clean() {
        let mut evidence = SkillEvidence::default();
        evidence.observe(Observation { passed: false, ..clean(0) });
        for variant in 1..6 {
            evidence.observe(clean(variant));
        }
        assert_eq!(evidence.suggest().practice, Practice::Reduce);
        assert_eq!(evidence.support(), Support::Guided);
        evidence.observe(clean(6));
        assert_eq!(evidence.suggest().practice, Practice::Sprint);
    }

    #[test]
    fn sprint_requires_repeated_varied_unassisted_independent_success() {
        let mut evidence = SkillEvidence::default();
        evidence.observe(clean(0));
        assert_eq!(evidence.suggest().practice, Practice::Repeat);
        for _ in 1..4 {
            evidence.observe(clean(0));
        }
        assert_eq!(evidence.suggest().practice, Practice::Vary);
        evidence.observe(Observation { independent: false, ..clean(1) });
        assert_eq!(evidence.suggest().practice, Practice::Independent);
        for variant in 2..8 {
            evidence.observe(clean(variant));
        }
        assert_eq!(evidence.suggest().practice, Practice::Sprint);
    }

    #[test]
    fn dimensions_and_recall_remain_separate_and_serializable() {
        let mut evidence = SkillEvidence::default();
        for variant in 0..4 {
            evidence.observe(Observation { seconds: 20.0, ..clean(variant) });
        }
        assert_eq!(evidence.suggest().practice, Practice::Vary);
        assert_eq!(evidence.transfer, CorrectnessEvidence::default());
        assert_eq!(evidence.delayed_recall, RecallEvidence::default());
        evidence.record_delayed_recall(true);
        let json = serde_json::to_string(&evidence).unwrap();
        assert_eq!(serde_json::from_str::<SkillEvidence>(&json).unwrap(), evidence);
        assert_eq!(serde_json::from_str::<SkillEvidence>("{}").unwrap(), SkillEvidence::default());
    }

    #[test]
    fn invalid_speed_cannot_unlock_sprint() {
        for seconds in [f32::NAN, f32::INFINITY, -1.0] {
            let mut evidence = SkillEvidence::default();
            for variant in 0..4 {
                evidence.observe(Observation { seconds, ..clean(variant) });
            }
            assert_eq!(evidence.duration.valid, 0);
            assert_ne!(evidence.suggest().practice, Practice::Sprint);
        }
    }
}
