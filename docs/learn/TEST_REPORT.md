# Test report — verified 2026-10-12

Compiler: rustc 1.99.0. Passing build settings: CARGO_BUILD_JOBS=2, CARGO_PROFILE_DEV_CODEGEN_UNITS=1, CARGO_PROFILE_DEV_DEBUG=0, CARGO_INCREMENTAL=0. Test commands and complete outputs are in validation-logs/.

## Passing new work

- Real-engine proof: create workbook, write range, assign reversed-factor formula, recalc, inspect 360, change price in an isolated clone, inspect 225, verify original remains 360.
- cargo test -p gridcraft-learn-core -p gridcraft-learn --lib: **10 passed, 0 failed**.
- Core six tests cover 20 families × 12 seeds × three changed-input probes; alternative formula acceptance; constant formula rejection; source tampering/errors; three cases × eight seeds with partial scores and remediation; checkpoint resume/version rejection; deterministic content/prerequisites/two dimensions.
- App four tests cover wrong answer → hint → correct answer → deduplication → restart → case; real XLSX export and engine reopen/recalculation; actual visible Check/Hint/Next clicks; F2 → typed formula → Enter in the real grid; local TCP HTTP requests/envelope decoding for Gemini, compatible and Ollama plus response schema rejection.
- cargo fmt --all -- --check: passed.
- cargo clippy -p gridcraft-learn-core -p gridcraft-learn --all-targets --no-deps -- -D warnings: passed.
- cargo build -p gridcraft-learn: passed; runnable Linux x86_64 optimized development executable produced.
- cargo check -p gridcraft: passed.
- xtask assets and layers: passed (18 registered crates, no layering violations). Tooling emits existing no-default-feature warnings, not validation errors.
- wasm32 check for learn-core and original gridcraft-web: passed. This does not imply the learning desktop host runs in the browser.
- Actual egui/wgpu offscreen screenshot rendered and visually inspected at 1280×820 using software Vulkan. Column clipping fixed and rerendered.

## Baseline failures

Before learning changes, cargo test -p gridcraft-engine --lib: **226 passed, 2 failed**:
1. io::tests::csv_save_keeps_the_separator_it_was_opened_with
2. tests::saving_an_opened_semicolon_csv_keeps_semicolons

Both expect 124412140710 while the implementation emits 1.24412E+11. Full upstream CI is not claimed green; no git commit was made because the upstream pre-commit gate is not satisfied.

Initial default compiler/linker attempts failed before application code. Single codegen unit fixed those failures. Intermediate API/egui integration errors were corrected; only final successful outputs support the passing claims.

## Not executed / not proven

Real live LLM quality, web learning UI/persistence, native window interaction on a real desktop, release-mode packaging, accessibility audit, long-running crash/large-file soak, full ten-scenario professional acceptance. Missing product features are listed in KNOWN_LIMITATIONS.md; no test assertion substitutes for those features.
