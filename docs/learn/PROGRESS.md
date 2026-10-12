# Progress — 2026-10-12 (v2)

Upstream SHA: 3926d21aa9a79d272b9f71ae60d67d7d7e5bd2c2. Nothing published or pushed.

## v2: what changed

- **Russian formula input** in the formula crate (`FormulaLocale::Ru`): ~130 function names, `;` separator, decimal comma, ИСТИНА/ЛОЖЬ, Ё folding, Cyrillic or Latin X in ПРОСМОТРX. The editor's autocomplete and signature help are Unicode-aware.
- **Learning core rewritten** (`crates/learn`): 54 skills in 11 units plus 6 cases, each skill generated at 4 staircase levels from a seed (layout seed + data seed). Grading re-evaluates the learner's formulas on two perturbed data sets; hotkey drills check recorded key combos; data drills check sort/filter/freeze/format/dedupe state; pivot-style answers are found anywhere on the sheet.
- **Adaptive loop:** staircase (2/2/3 clean wins to climb, 2 misses to step down), FSRS-4.5 per skill, lesson planner (remediation → due reviews → one mix → frontier), misses re-queued one level easier, unit and skill unlocks.
- **Gamified app:** path home with zigzag nodes and level rings, one big button (Enter) starts a lesson, auto-check when the yellow cells are filled, feedback banner with Enter to continue, combo, XP, daily goal, streak, sounds synthesized in code, confetti, original mascot. Cases with a brief e-mail; interview mode with timer and no hints; report with remediation.
- Save format v2; v1 progress is kept as `progress.old.json`.

## Verified

- `gridcraft-learn-core`: 13 tests (every skill × 4 levels × 3 seeds solvable by its reference and not passing untouched; cases solvable; typed numbers and missing `$` rejected; equivalents accepted; staircase, unlocks, streak, interview remediation, save round trip).
- `gridcraft-learn`: 9 tests through the real egui UI (Enter starts a lesson, auto-check, Enter advances, a whole lesson reaches the done screen, a Russian formula typed in the grid passes, shortcut recording, save/resume of the open exercise, interview grading, every screen renders).
- `gridcraft-formula`: Russian input/display round trips.
- clippy `-D warnings` and fmt clean on the touched crates. Offscreen screenshots of every screen checked (lavapipe).

## Next

Real-desktop smoke test by the learner; more case variety; per-learner FSRS weight fitting once there is enough history; web host.

## v3 — Taskathlon

- **SQL:** 16 skills (`sql.*`) on an in-memory SQLite (rusqlite, bundled). Read-only guard, 2 s timeout, 2000-row cap. The result set is graded against the reference and re-run on two perturbed data seeds.
- **Five new work cases:** supplier comparison, stock balances, budget vs actual, vacation schedule, retail sales.
- **Express test:** 15 timed multiple-choice questions (Excel, SQL or mixed), each with its own timer. Points for speed, a combo bonus, and misses sent to remediation.
- **Sprint:** 10 real tasks against a stopwatch, +10 s per miss, gold/silver/bronze against par, personal records per skill and for the mix.
- **Motivation:** speed tiers (Lightning/Fast) with bonus XP, combo bonus, gems, daily quests, streak freeze, streak wager, 18 achievements, toasts and pop-ups, a mascot greeting chosen from state, a progress page (14-day bars, 5-week calendar, badges, records), and an optional daily desktop reminder (systemd user timer and `notify-send`).
- **Adaptivity:** "I already know this" test-out per unit; a first clean, fast answer jumps to level 3; fast clean answers count double toward the next level.
- **Renamed** to Taskathlon with its own colour palette.
