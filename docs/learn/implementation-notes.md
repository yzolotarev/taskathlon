# Curriculum update implementation notes

## Approved scope

Add a coherent procurement scenario route on the existing Rust spreadsheet
engine, preserve the old course and saved progress, then generalise procedural
practice after a working vertical slice. Do not create a replacement JS engine.

## Scope decision A/B/C (this increment)

The project adopts a ladder from micro-actions to real work, with three
mechanisms: FSRS retention (implemented), scaffolding + skill graph (partially
present), and adaptive mastery learning (evidence implemented, corrective
insertion in progress). Three ways to progress the ladder were considered:

- **Option A (adopted):** harden the single continuous supply ladder into a real
  transferable route — seeded layout/data variation so positions cannot be
  memorised, corrective practice on error inside the flow, and a composite
  manager brief (seed: Датчик 6×8500, Кабель 30×1200, Контроллер 2×42000,
  Крепёж 100×150, Блок питания 4×12500, total 236000; on the change request
  cable 30→45 the total becomes 254000, recomputed by formula, not by hand).
- **Option B (deferred):** the five-crossing-ladders system.
- **Option C (deferred):** the explicit atom/skill dependency graph.

Each of B and C is deferred until after the interview deadline and is **not
started work**. Do not describe either as implemented.

Level-3 gameplay "juice" (green flash, series/accuracy HUD, XP) is explicitly
out of scope for this increment.

## Work streams

- Curriculum sequence and seeded procurement case.
- Ladder hardening (option A): seeded layout/data variation, corrective
  insertion inside the flow, composite manager brief.
- Continuous desktop scenario, quiet feedback and restart behavior.
- Grading policy, real pivot checks and bounded multi-sheet saves.
- Separate learning evidence and conservative adaptation recommendations.
- Integration, regression tests and honest release notes.

## Confirmed corrections

- A shown answer is assistance and no longer triggers independent fast-track or
  speed bonuses. Two introductory repetitions still open supported practice.
- Lesson planning preserves remediation beyond its first two entries.
- Review/mixing stays within the current Excel, SQL or accounting domain.
- Frontier prioritises level 3 and then offers remaining level 4 practice.

## Deviations and constraints

- Prowl impact was attempted but no `.prowl` workspace exists. Graph queries and
  targeted source reads were used instead; no repository-wide index was created.
- The first test invocation timed out during dependency compilation after waiting
  for Cargo's build-directory lock; this is not a passing or failing test result.
- Flow cannot be certified from unit tests or a graph. Desktop interaction and
  unfamiliar-task transfer remain explicit human validation gates.
- Adding multi-sheet save support does not by itself implement multi-sheet
  grading or scenario content; these capabilities must be reported separately.
- Corrective insertion on error is **in progress** and not yet validated.
  Seeded layout/data variation and the composite brief under option A are part of
  this increment and still need executable and desktop evidence before being
  claimed as done.
- Option B (five-crossing ladders) and option C (explicit atom/skill dependency
  graph) are **deferred** until after the interview deadline. They are not
  implemented, not partially implemented, and must not be reported as done.

## Validation (final)

Commands run from `/home/y/taskathlon` with
`CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_CODEGEN_UNITS=1 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0`:

- `cargo test -p gridcraft-learn-core -p gridcraft-learn -p gridcraft-tr --lib`
  — 58 passed / 0 failed (17 app, 38 core, 3 translation).
- `cargo clippy ... --all-targets --no-deps -- -D warnings` — clean.
- `cargo fmt --all -- --check` — clean.
- `cargo build -p gridcraft-learn` — succeeded; runnable binary produced.

New tests covering this update include the continuous supply route (first typing,
real fill-down, altered-order rejection, every-step restart, explicit completion),
the procurement case (seeded, independent arithmetic, real pivot definition),
pivot/selection/source-policy checks, bounded multi-sheet saves, evidence
recommendations, contextual lesson planning longer, and the shown-example rule.

No commit or merge was made. Human validation still required: desktop interaction
in a real window, and unfamiliar-task transfer (and Microsoft Excel parity if
available). These are listed in `SCENARIO_SMOKE_TEST.md`.

The ladder-hardening increment (option A: seeded layout/data variation,
corrective insertion, composite brief) was not covered by the run above. Its
executable and desktop evidence is **pending**; the 58-test figure is the
preceding checkpoint, not a result for this increment. No cargo run was performed
while updating these documents, so no new pass counts are claimed here.
