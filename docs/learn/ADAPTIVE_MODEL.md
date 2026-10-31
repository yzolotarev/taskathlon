# Adaptive model — current implementation and historical notes

The text below describes the initial bootstrap and is retained as historical
context, not the current scheduler contract. The current `crates/learn/src/fsrs.rs`
implements FSRS-4.5 with published default parameters; these are not fitted to
this learner and do not measure professional competence. `profile.rs` schedules
eight-slot lessons with remediation, due reviews, contextual mixing and frontier
practice. The frontier prefers independent level 3, then extends skills to level
4. Copying a shown example does not grant speed bonuses or independent fast-track;
two successful introductory repetitions open supported level 2 practice.

Step-down after a miss (2026-10-18): the next task of that skill is served one level
easier exactly once (a lesson retry is that step already); solved, the learner is back on
level. Two misses in a row lower the staircase one level, and that drop is not stacked
with the easier task. Before this, the evidence window served the lower level for six
tasks that did not count, and stacked with the drop: one miss cost 10 tasks from level
3 to 4 instead of 3, two misses 13 with six copy-the-example tasks. Now: 5 and 7, never
below level 2 for a learner who already worked independently (regression test
`a_miss_costs_one_easier_task_not_a_long_detour`).

See [Curriculum update](CURRICULUM_UPDATE.md) for the scenario route contract and
the distinction between procedural practice, retention and transfer evidence.

## Historical bootstrap (superseded)

No dataset has been collected or optimized. This implementation is not FSRS and does not report probabilities. The model deliberately keeps procedural fluency and task autonomy separate from review timing.

Selection: the most recent failed criterion's skill first, then the oldest overdue skill, then the first unlocked skill with fewer than two successes. Prerequisites currently unlock after two successes. Every new instance increments its seed. After each fifth solved exercise, a method-selection drill is labelled autonomous; real multi-criterion open assignments are separately selectable.

Intervals: incorrect criterion = review after five minutes plus immediate remediation; assisted success = one hour; unassisted consecutive success = 1, 2, 4, 8, 16, then 32 days. Incorrect work resets the streak. Assisted work does not count as proof of autonomy. Due timestamps, attempt results, time and hint use are saved.

Fluency = observed success fraction × 0.7 when the latest attempt exceeds 120 seconds × 0.7 when hints were used. Autonomy changes only on open attempts (+0.2 for correct unassisted work, −0.1 otherwise). These are transparent bootstrap signals, not estimates of real job competence. Their main useful property is that errors actually change subsequent practice. Known weaknesses: repeated/assisted success unlocks, speed based on the latest observation, limited context diversity, wall time includes reading/idle time.

Next: integrate pinned fsrs-rs memory state for retention, retain procedural metrics outside it, require independent successes in varied contexts for mastery, use diagnostic drills for uncertain case attribution, and validate transfer in new cases. Do not train on fabricated observations.
