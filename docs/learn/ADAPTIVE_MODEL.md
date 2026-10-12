# Bootstrap adaptive model

No dataset has been collected or optimized. This implementation is not FSRS and does not report probabilities. The model deliberately keeps procedural fluency and task autonomy separate from review timing.

Selection: the most recent failed criterion's skill first, then the oldest overdue skill, then the first unlocked skill with fewer than two successes. Prerequisites currently unlock after two successes. Every new instance increments its seed. After each fifth solved exercise, a method-selection drill is labelled autonomous; real multi-criterion open assignments are separately selectable.

Intervals: incorrect criterion = review after five minutes plus immediate remediation; assisted success = one hour; unassisted consecutive success = 1, 2, 4, 8, 16, then 32 days. Incorrect work resets the streak. Assisted work does not count as proof of autonomy. Due timestamps, attempt results, time and hint use are saved.

Fluency = observed success fraction × 0.7 when the latest attempt exceeds 120 seconds × 0.7 when hints were used. Autonomy changes only on open attempts (+0.2 for correct unassisted work, −0.1 otherwise). These are transparent bootstrap signals, not estimates of real job competence. Their main useful property is that errors actually change subsequent practice. Known weaknesses: repeated/assisted success unlocks, speed based on the latest observation, limited context diversity, wall time includes reading/idle time.

Next: integrate pinned fsrs-rs memory state for retention, retain procedural metrics outside it, require independent successes in varied contexts for mastery, use diagnostic drills for uncertain case attribution, and validate transfer in new cases. Do not train on fabricated observations.
