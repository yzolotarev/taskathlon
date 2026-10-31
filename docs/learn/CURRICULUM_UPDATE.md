# Scenario-based procurement curriculum

## Learning contract

Practise in the real embedded spreadsheet, without opening external workbooks.
The adopted model is a **ladder from micro-actions to real work**: small
operations first, then combined work, then an unfamiliar composite brief. A
single fixed-length introductory session followed by unrelated exercises is not
the model. A learner advances along the ladder only as logged evidence allows,
and no session length is a deadline or a claim of professional readiness.

The procurement route is additive. Existing practice, SQL and accounting remain
available; neither XP nor a fast copied example demonstrates independent mastery.

## Ladder mechanisms (implemented vs deferred)

1. **Retention — FSRS.** `crates/learn/src/fsrs.rs` schedules when a known skill
   should next be reviewed. This is **implemented**. It models memory of a
   procedure, not mastery of it: a due card prompts practice, it is not proof of
   competence, and the published parameters are not fitted to this learner.
2. **Gradual difficulty — scaffolding.** Levels 1..4 with fading support are
   **partially implemented**. The explicit atom/skill dependency graph that would
   make the ladder data-driven is **not yet implemented** and is deferred.
3. **Adaptive mastery learning — evidence.** `crates/learn/src/evidence.rs`
   records correctness, assistance, duration and transfer evidence and returns
   conservative practice recommendations. This is **implemented**. Corrective
   insertion on an error, inside the same flow, is **being added now** (in
   progress, not yet reported as done).

No level-3 gameplay "juice" (green flash, series/accuracy HUD, XP) is in scope.

## Scope decision: option A

Harden the single continuous supply ladder into a real transferable route rather
than widening to a new system:

- **Seeded layout/data variation** so the learner cannot memorise positions: the
  same skill recurs with different cell positions and different numbers across
  attempts.
- **Corrective practice on error, inside the flow**, returning to the saved work
  without discarding it.
- **A composite manager brief** that recombines the ladder's operations, using
  this exact seeded case:

  | Position | Quantity × unit | Line total |
  | --- | --- | --- |
  | Датчик | 6 × 8500 | 51000 |
  | Кабель | 30 × 1200 | 36000 |
  | Контроллер | 2 × 42000 | 84000 |
  | Крепёж | 100 × 150 | 15000 |
  | Блок питания | 4 × 12500 | 50000 |
  | **Total** | | **236000** |

  The subsequent change request raises the cable quantity from 30 to 45; the
  total must become **254000** (cable line becomes 54000, +18000). The change
  must be recalculated by formula, not by hand-editing the total.

Deferred until after the interview deadline: the **five-crossing-ladders system**
and the **explicit atom/skill dependency graph**. These are not started work and
must not be reported as implemented.

## Sequence

1. Enter and correct an order: navigation, Enter/Tab, editing and undo.
2. Calculate one position: arithmetic, references and recalculation.
3. Process the order: selection, relative references, copying and fill down.
4. Check the total: SUM, range boundaries and missing rows.
5. Apply common terms: percentages, rounding, absolute references and F4.
6. Inspect the register: whole-row sorting, filtering and number formats.
7. Monitor deliveries: dates, IF, AND/OR and boundary cases.
8. Answer supplier questions: SUMIF, COUNTIF and contextual aggregates.
9. Combine conditions: supplier, status and period with SUMIFS.
10. Join catalog and orders: exact lookup, fixed ranges and missing articles.
11. Build a real pivot: supplier by status, sum/count, filters and refresh.
12. Complete an independent brief and handle a subsequent change request.

Mixed references belong in a two-direction copying task, not as a prerequisite
for every procurement lookup. Keyboard practice appears where the operation is
needed, rather than being postponed to a separate late unit.

## VLOOKUP + IFERROR composition ladder

Nested formulas are not introduced as one memorised string. The global skill
sequence now inserts three smaller nodes immediately after exact-match VLOOKUP:

1. `lookup.error_observe`: enter VLOOKUP for a known code and an absent code; see
   the real `#N/A` rather than masking it.
2. `lookup.iferror.wrap`: the VLOOKUP result is already present; practise only
   wrapping that result in IFERROR and compare the found/missing rows.
3. `lookup.iferror.combine`: build the nested lookup+fallback on varied order
   rows, then transfer to a changed layout/data variant.

Failed answers in these three skills do not reveal the entire nested solution in
the feedback banner. The worked-example level can show the expression before a
practice attempt; levels 2–4 fade that support. Each correct repetition advances
the massed-practice counter once: five clean attempts at supported levels 1 and
2, then eight independent level-3 attempts before level 4. A speed bonus never
counts as two repetitions for these composition skills. Other skills keep their
existing staircase thresholds. This is a conservative instructional rule, not
an empirical claim that five/eight repetitions are universally optimal.

## Live-data step (implemented 2026-10-15)

A clean drill proves the syntax; work data is messier. After a passed drill of a
supported skill at level 2 or higher, the lesson continues once per skill with a
**live-data step**: the same kind of question about a hand-kept supply register,
opened below the solved drill in the same workbook (the view scrolls only as far
as needed, so both stay visible when they fit). No banner or separate screen sits
between them.

- Supported skills: СУММ, СРЗНАЧ, МИН/МАКС, СЧЁТ, СУММЕСЛИ, СЧЁТЕСЛИ, the
  "greater than" condition, СУММЕСЛИМН, СЧЁТЕСЛИМН, СРЗНАЧЕСЛИ, multiplication and
  fill-down (line totals), ВПР / ПРОСМОТРX / ИНДЕКС+ПОИСКПОЗ (catalog lookup).
- Entry errors used: stray spaces and a different spelling of a supplier, a status
  spelled differently, «12500 тг» / «5 шт» in number cells, a number stored as
  text, a subtotal row inside the data, a blank row, an order typed twice, an
  article with a space or without its dash. Harmless noise (letter case) is mixed
  in from tier 2, because noticing what does *not* matter is part of the skill.
- Dirt tiers per skill (1..4): tier N carries N kinds of dirt. The answer cell fades
  from highlighted (1), to labelled (2), to self-placed next to a «Ответ» label
  (3–4). Two unassisted passes raise the tier; two misses in a row lower it.
- Grading is an independent Rust oracle over the clean truth and needs a formula
  that references cells. Cleaning the source by hand is allowed. Every subset of
  untreated dirt has its wrong answer precomputed, so a miss names the trap and
  its rows ("строка 15: «Вектор» — для СУММЕСЛИ это другое название").
- A missed register comes back later in the lesson as a fresh register at the
  same tier. The banner's «ИСПРАВИТЬ» keeps the same register for an in-place fix.
- Live attempts move their own tier and add transfer evidence for the skill; they
  never move the clean-drill staircase or its FSRS schedule.
- Tests: the trap model is checked against the real engine on 1920 registers
  (straightforward formulas over dirty data must give exactly the predicted wrong
  answer); every tier must contain as many real traps as it promises; cleaning by
  hand then a plain formula passes at every tier; ignoring the dirt fails with a
  named trap; typed numbers and `=143500` are rejected; no generated cell overlaps
  another; the app keeps the drill in the book and opens the register below it.

Not done in this increment: the multi-sheet procurement case (orders, supplier
price list, stock) and the written conclusion for the manager. A formula such as
`=0*A1+<answer>` would pass, because only the presence of a cell reference is
checked; this is a known gap, not a realistic learner strategy.

## Learner feedback fixes (2026-10-18)

Three screens from a real learning session showed the learner stuck without a way
forward. Fixed:

- **Hints never repeat the screen.** At level 1 the formula is already shown, so the
  hints were the topic idea, a generic template and the same formula again. Now
  level 1 explains the shown formula instead («Что значит каждая часть: E2 - что
  ищем, A2:C6 - таблица…», built from the skill template and the real cells) and
  how to type it. Levels 2-4 go idea → template → that breakdown → answer.
- **A slipped reference is named.** If the learner's formula equals the reference
  except for one cell address (the neighbouring row's code), the check says
  «Формула в F2 берёт E3, а здесь нужна E2» instead of a bare «#Н/Д».
- **`lookup.error_observe` shows both cells.** Two yellow cells, each with its own
  worked formula; the auto-check waits for both (error-showing cells now count as
  answer cells). Its hints explain why one cell gets a price and the other #Н/Д.
- **Spotting errors is a graded skill in live data.** Tier 1 always carries one
  error you can see (a «тг», a subtotal row, a name written differently), tier 2
  adds the next kind (often an invisible one), tiers 3-4 add the rest at random.
  For the catalog lookup, tier 1 pre-fills a «Сумма строки» column so the broken
  article shows #Н/Д in its own row, and the answer is a plain СУММ; tier 2 asks the
  learner to build that column (formula in hint 2); from tier 3 nothing is laid out.
- **Choosing the function is a skill.** `choose.totals` (end of unit 4, supply
  stage 08) and `choose.lookup` (end of unit 5, supply stage 10) ask a question
  about one invoice table without naming the tool. Level 1 lists three candidates
  (the right one and the two most confusable with it), level 2 asks with cue words,
  level 3 in a manager's words, level 4 asks three questions at once. Hint 1 quotes
  the cue words of the question actually asked. Any formula with the right value
  passes; the live re-check on other data rejects a look-alike that agrees by chance.

## «Проверь себя»: control formulas (2026-10-18)

A learner should be able to prove with a formula that a table is clean, not only
spot errors by eye. Each error kind gets its own check, taught right after the
function it uses; levels 1-2 use a visible error, levels 3-4 an invisible one that
only the formula sees. The learner writes the check, then types the broken row.

- `check.counttext` (unit 2): СЧЁТЗ − СЧЁТ finds numbers stored as text («тг», then a
  1С-style «12 500»).
- `check.reconcile` (unit 4): grand total minus the total of every proper status;
  the difference is exactly the amount of the misspelled row.
- `check.catalog` (unit 5): СЧЁТЕСЛИ of each order code over the catalog; 0 marks
  the code VLOOKUP will not find (no dash, then a trailing space).
- `check.spaces` (unit 6): ДЛСТР − ДЛСТР(СЖПРОБЕЛЫ) per row.

A test proves each control flags exactly the broken row at every level.

Live data uses the same checks: hint 2 shows the control formula for the errors in
the register; an unrecognised wrong answer gets it instead of «сверь вручную»; and
a formula whose condition is taken from a broken cell (e.g. `…;H4)` where H4 is
«Получено») is named: «Условие взято из H4, а там ошибка ввода». The feedback
banner now wraps long messages instead of running under its buttons.

## Pivot ladder and supply-assistant tasks (2026-10-18)

- `data.pivot` now always requires a real pivot object (SUMIF summaries no longer
  pass): level 1 one field in Rows with Sum, level 2 supplier × status, levels 3-4
  a manager's question that decides the layout and Sum vs Count. The pivot may sit
  on any sheet (Insert → PivotTable creates a new one in front); task addresses are
  resolved on the exercise sheet by name. Feedback names the first difference
  (no pivot, wrong Rows/Columns, «Количество» instead of «Сумма» and how to switch).
- The PivotTable Fields pane is translated to Russian Excel wording.
- `supply.need` (unit 3, supply stage 05): need = per-device norm × production plan
  ($), to order = МАКС(0; need − stock − in transit); one component is always
  covered so the zero floor matters.
- `supply.overdue` (unit 7, supply stage 07): ЕСЛИ(И(status ≠ «Получен»; due <
  today); «Просрочено»; ""); late-but-received and not-yet-due rows always appear.

Extras (`choose.*`, `check.*`, `supply.*`) never close a unit or the next core skill.
The autopilot («Продолжить») heads for a supply-assistant goal list first (СУММЕСЛИ,
запасы, ВПР, сверка, фильтр, сводная, просрочки, …): a locked goal is reached through
the first core skill of its unit that still holds it back, so nothing is skipped.

Known gap: pivot output captions come from the engine in English («Row Labels»,
«Sum of …», «Grand Total»), unlike Russian Excel.

## Report assembly unit (implemented 2026-10-15)

Unit 13 «Сборка отчёта» drills building one tidy report sheet («Отчёт», the first
tab) from messy source sheets in the same book. It opens once unit 2 (fill-down and
`$`) is done. Eight skills, each with the usual four levels and fresh data on every
attempt:

1. Перенос по столбцам: other header names, other column order, extra columns,
   from level 4 a title above the export.
2. Числа из текста: «5 шт», «1200 тг», digits stored as text.
3. Даты в один формат: text dates, dates in words («3 марта»), short years, real
   dates shown as yyyy-mm-dd (harmless); level 4 adds days in transit.
4. Названия по справочнику: supplier spellings against a «Справочник» sheet.
5. Цена с другого листа: lookup across sheets; from level 3 the article is not the
   price list's first column; from level 2 one article carries a trailing space.
6. Два источника в один список: two managers' sheets with different column orders;
   from level 3 one order appears in both.
7. Оформление отчёта: bold and frozen header, thousands format, sort by date, a
   «Итого» row with a formula, added level by level.
8. Отчёт целиком: all of the above together.

Grading compares the report row by row (matched by order number, any row order)
with an independent truth; the method is free. A miss names the order, the column
and, when the value came dirty, the source sheet and row it came from. A pasted
header row or a repeated order is reported as such; «Итого» rows are ignored.

Engine fixes made for this unit, because they would have taught habits that differ
from Russian Excel: dotted dates are day-first (`12.03.2026` is 12 March, also in
ЗНАЧЕН/ДАТАЗНАЧ), and with the Russian interface typing `12.03` gives 12 March of
the current year instead of the number 12.03.

Reviews of skills already at level 3 or higher now go straight to a live-data
register instead of a clean warm-up; a passed register answers the due review.

## Assessment rules

- Distinguish three kinds of evidence: **result checks** (the computed value is
  right), **operation checks** (the intended procedure was actually used) and
  **transfer evidence** (the method works on an unfamiliar layout or case). A
  correct aggregate is not proof of constructing a pivot; a recorded shortcut is
  not proof of using it correctly in the intended context; neither is proof of
  transfer.
- Shown examples are assistance, even if no hint button was pressed.
- Permit source edits explicitly requested by the brief; immutable inputs still
  need integrity checks.
- Failed probe generation must not silently count as passed evidence.
- Prefer independent Rust expectations for money, grouping and status logic.
- Record precision, assistance, time and transfer separately. Never estimate a
  hiring probability from local scores.

## Runtime contract

Keep the workbook between scenario steps. Do not consume the Enter that commits
a formula again as a next-step command. Feedback is local during practice; a
compound assignment is submitted explicitly. Pause must stop the practice clock.
Correction branches must return to the saved work without overwriting it.

Persist scenario version, stage, variants, assistance, completion and workbook
atomically or with a checked compatibility contract. Preserve old progress.
Multi-sheet assessment needs sheet-qualified addresses as well as bounded save
validation; allowing multiple sheets in a save alone does not implement it.

## Release gates

1. Real UI path: input, selection, fill, error correction and submission.
2. Reference and alternative solutions; untouched and hard-coded answers fail.
3. Restart at every step without loss or duplicate rewards.
4. New-layout task and delayed retest without hints.
5. Where available, a final Microsoft Excel transfer check: GridCraft is not a
   guarantee of identical desktop Excel behavior.

## Implementation honesty

This document describes the target contract. Implemented scope must be reported
from executable tests and actual UI behavior, not inferred from this sequence.
The full adaptive scenario system, operation telemetry across all disciplines,
professional readiness validation and multi-sheet probes are separate release
gates, not features established merely by adding a route definition.
