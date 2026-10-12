# Implementation plan

1. M0: read repository instructions and public references; pin upstream SHA; install Rust; run engine baseline, desktop/web checks.
2. M1: run crates/engine/examples/learn_proof.rs proving creation, range writes, formula, recalculation, result inspection, changed-input recalculation and isolation. Only after this passes write the learning integration.
3. M1–M2: seeded content, criteria and typed evaluation; real SheetApp practice panel with Check/Hint/Next; test incorrect/correct/alternative/constant answers.
4. M3–M5: prerequisite graph, adaptive selection, honest bootstrap scheduling, durable versioned progress and unfinished workbook.
5. M6–M8: open multi-criterion cases with partial credit, targeted remediation and transfer retest.
6. M9–M10: optional privacy-controlled AI providers and validated authoring pipeline; local hints remain available offline.
7. M11–M12: original puzzles, UI/E2E checks, wasm, quality gates and release documentation.

Do not skip correctness or label pending stages complete. Record executable evidence in TEST_REPORT.md. Resume from PROGRESS.md.

Verified checkpoint: M1/M2 and selected M3–M9 subsets implemented. See PROGRESS.md for exact scope; remaining numbered steps are backlog, not completed work.
