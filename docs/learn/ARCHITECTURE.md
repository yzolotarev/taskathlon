# Architecture — discovery

Base SHA: 3926d21aa9a79d272b9f71ae60d67d7d7e5bd2c2.

GridCraft model stores a Workbook with Arc sheets and copy-on-write bands. Session::execute dispatches CommandSpec entries; edits journal, record undo snapshots and recalculate dependencies. DocState::new performs full recalculation. SheetApp owns the swappable egui frontend. CLI/MCP/control share command IDs. XLSX/CSV are imported by engine::io; save_bytes writes supported formats. Workbook JSON supports serde; formula AST reparses on load.

Implemented integration: a toolkit-free learning module above engine (L6) and a separate application host (L7) that reuses SheetApp. No handwritten JS/TS, no replacement engine, no lower-layer UI dependencies. Register new packages in xtask layers table. Keep grading detached from learner edits. Version seeded content and progress; restore unfinished workbook and attempt status together. Local help is authored content, never described as an LLM.

Known upstream limitations: incomplete Excel conformance, float equality, structural spill changes, incomplete XLSX roundtrip for unmodelled features, incomplete accessibility. Use small synthetic workbooks and supported operations; no promise of full Excel parity.

## Implemented boundaries

crates/learn (gridcraft-learn-core, L6) depends only on engine and serialization/error crates. It owns Family, Exercise, Criterion, independent Rust oracles, probe generation, Evaluation, Progress and Checkpoint. apps/gridcraft-learn (L7) hosts SheetApp, native atomic persistence and UI; ai.rs owns native TLS HTTP adapters. Engine and UI source are unchanged; one engine example is added. xtask registers both new packages and includes learn-core in its wasm check. Learning controls are currently local application actions; they are not exposed through GridCraft MCP/control commands.

AI runs on a worker thread; responses carry an exercise seed so old replies cannot affect a later task. No tool execution or spreadsheet mutation is exposed. Keys are read from the process environment and are not stored in checkpoints or shown in errors. TLS validates certificates; redirects are disabled. Context ≤16 KiB, response body ≤64 KiB, validated hint JSON ≤4 KiB; timeout 20 s.
