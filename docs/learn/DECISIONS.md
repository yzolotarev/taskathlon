# Decisions

- Reuse the Rust engine and egui frontend. Evidence: Session::execute, DocState::new, SheetApp::ui.
- Numerical oracles must be independent from learner formulas. Probes operate on COW copies. Proof source: crates/engine/examples/learn_proof.rs.
- Start with honest bootstrap heuristics for procedural fluency. FSRS models memory, not procedural transfer. FSRS integration is pending; do not call a heuristic FSRS.
- No Gemini Web gateway in the main codebase. N1Moo/web2api is MIT and exposes compatible HTTP APIs, but upstream web protocol stability and service permission have not been established. HanaokaYuzu/Gemini-API is AGPL-3.0, incompatible with the project's prohibition on copying AGPL code; no code imported.
- No public publishing or remote repository edits. Use local branch learn/initial.

- Native tutor uses ureq 3.4 with Rust TLS, bounded output and no redirects. All three envelopes pass local HTTP tests; no real model availability assumed.
- Native shell uses the same wgpu feature set as GridCraft to avoid breaking eframe trait signatures through workspace feature unification.
- Scoped native clock helpers are off wasm and allow only the repository clock lint at those native calls.
- No commit: upstream baseline test failures prevent the required whole-repository quality gate.
