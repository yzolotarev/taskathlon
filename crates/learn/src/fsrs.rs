//! FSRS-4.5 memory model with its published default parameters, applied per skill.
//!
//! FSRS models recall of facts. Here a "review" is solving a fresh, never-seen variant of the
//! skill, so it measures whether the procedure still works. The parameters are the library
//! defaults, not fitted to this learner; the scheduler says so in the UI.

use serde::{Deserialize, Serialize};

const W: [f64; 17] =
    [0.4872, 1.4003, 3.7145, 13.8206, 5.1618, 1.2298, 0.8975, 0.031, 1.6474, 0.1367, 1.0461, 2.1072, 0.0793, 0.3246, 1.587, 0.2272, 2.8755];
const DECAY: f64 = -0.5;
const FACTOR: f64 = 19.0 / 81.0;
pub const RETENTION: f64 = 0.9;
const DAY: f64 = 86_400.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grade {
    Again = 1,
    Hard = 2,
    Good = 3,
    Easy = 4,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Card {
    /// Days until recall probability falls to 90%.
    pub stability: f64,
    /// 1 (easy) .. 10 (hard).
    pub difficulty: f64,
    pub last: u64,
    pub due: u64,
    pub reps: u32,
    pub lapses: u32,
}

fn w(i: usize) -> f64 {
    W.get(i).copied().unwrap_or(1.0)
}

impl Card {
    pub fn is_new(&self) -> bool {
        self.reps == 0
    }

    /// Probability the skill is still performed correctly at `now`.
    pub fn retrievability(&self, now: u64) -> f64 {
        if self.is_new() || self.stability <= 0.0 {
            return 0.0;
        }
        let t = now.saturating_sub(self.last) as f64 / DAY;
        (1.0 + FACTOR * t / self.stability).powf(DECAY)
    }

    pub fn review(&mut self, grade: Grade, now: u64) {
        let g = grade as i32 as f64;
        if self.is_new() {
            self.stability = w(grade as usize - 1).max(0.1);
            self.difficulty = (w(4) - (g - 3.0) * w(5)).clamp(1.0, 10.0);
        } else {
            let r = self.retrievability(now);
            let d0_good = w(4);
            let next_d = self.difficulty - w(6) * (g - 3.0);
            self.difficulty = (w(7) * d0_good + (1.0 - w(7)) * next_d).clamp(1.0, 10.0);
            self.stability = if grade == Grade::Again {
                (w(11) * self.difficulty.powf(-w(12)) * ((self.stability + 1.0).powf(w(13)) - 1.0) * (w(14) * (1.0 - r)).exp()).max(0.1)
            } else {
                let hard = if grade == Grade::Hard { w(15) } else { 1.0 };
                let easy = if grade == Grade::Easy { w(16) } else { 1.0 };
                self.stability
                    * (w(8).exp() * (11.0 - self.difficulty) * self.stability.powf(-w(9)) * ((w(10) * (1.0 - r)).exp() - 1.0) * hard * easy + 1.0)
            };
        }
        if grade == Grade::Again {
            self.lapses = self.lapses.saturating_add(1);
        }
        self.reps = self.reps.saturating_add(1);
        self.last = now;
        // A failed skill comes back in the next lesson; otherwise at 90% predicted recall.
        let days = if grade == Grade::Again { 0.0 } else { self.stability / FACTOR * (RETENTION.powf(1.0 / DECAY) - 1.0) };
        self.due = now.saturating_add((days.clamp(0.0, 365.0) * DAY) as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intervals_grow_with_success_and_reset_after_failure() {
        let mut c = Card::default();
        c.review(Grade::Good, 0);
        let first = c.due;
        assert!((2 * 86_400..=5 * 86_400).contains(&first), "{first}");
        c.review(Grade::Good, c.due);
        assert!(c.due - c.last > first, "second interval grows");
        let before = c.stability;
        c.review(Grade::Again, c.due);
        assert!(c.stability < before);
        assert_eq!(c.due, c.last);
        assert!((Card::default().retrievability(0)).abs() < 1e-9);
    }
}
