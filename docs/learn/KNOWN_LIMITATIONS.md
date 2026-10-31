# Known limitations (v2)

- **1С is not emulated.** The accounting unit trains the chart of accounts, journal entries (Дт/Кт), VAT, payroll/НДФЛ, insurance contributions and a trial balance (ОСВ) in the spreadsheet. For the 1С interface itself, use the free «1С:Предприятие — учебная версия».
- **Training tax rates are simplified:** НДФЛ 13 %, VAT 22 %, contributions 30 % as one rate. Progressive НДФЛ and reduced contribution rates are not modelled. These exercises are not tax advice.
- FSRS uses the published default weights, not weights fitted to this learner.
- Cases are single-sheet tasks with numbered deliverables, not multi-file simulations. Grading accepts any method for pivot-style answers, but per-row answers must be in the yellow cells.
- The ribbon's File menu has no file-picker services in this host; exercises are not meant to be saved as separate files.
- Unchanged from v1: AI help is optional, consent is per launch, live models were not tested; one running app per profile; 3-second autosave.
- Upstream baseline: `gridcraft-ui-egui` test `i18n::tests::translations_are_unique_and_preserve_unknown_text` fails on a missing French string («Different First Page»), unrelated to the Russian changes; two upstream CSV engine tests also fail as before.
- The Linux binary is an optimized x86_64/glibc build, not a signed package. It was rendered offscreen; a real-desktop launch is the learner's first check.

## v3

- SQL runs on SQLite. Dialect differences from PostgreSQL or MS SQL (`TOP`, `ILIKE`, date functions) are explained in hints but not emulated. SQL is unavailable in the web build.
- Reminders need systemd user services and `notify-send`. Without them the settings show a crontab line instead.
- Gems, quests and wagers are local and only motivational; nothing is sent to the network.
- Tax rates in the accounting tasks are simplified for training.
- The accounting module is a simplified clean-room ledger; see docs/ledger/PLAN.md for what is simplified.

## Live data (2026-10-15)

- Live-data texts are Russian only; the English interface switch does not translate them.
- One register per task on one sheet. The multi-sheet case (orders + price list + stock) is not built yet.
- A self-placed answer (tiers 3–4) must sit right of, or up to two rows below, a cell reading «Ответ».
- A formula is required, but only the presence of a cell reference is checked: `=0*A1+<answer>` would pass.
- When the register opens below a drill, its column widths replace the drill's, so a solved drill's long labels may be clipped.
- Report assembly (unit 13) is Russian-only and uses sheets of one book, not separate files.
- The engine still reads slash dates month-first (`12/03/2026` is 3 December) and does not read a decimal comma inside text (`ЗНАЧЕН("1200,5")` is an error). The exercises avoid both forms.
