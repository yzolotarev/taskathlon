# Known limitations (v2)

- **1С is not emulated.** The accounting unit trains the chart of accounts, journal entries (Дт/Кт), VAT, payroll/НДФЛ, insurance contributions and a trial balance (ОСВ) in the spreadsheet. For the 1С interface itself, use the free «1С:Предприятие — учебная версия».
- **Training tax rates are simplified:** НДФЛ 13 %, VAT 20 %, contributions 30 % as one rate. Real 2025–2026 rules (progressive НДФЛ, VAT 22 % from 2026, reduced contribution rates) are not modelled.
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
