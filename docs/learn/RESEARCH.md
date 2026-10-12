# Public source research — 2026-10-12

## GridCraft
Pinned checkout and AGENTS.md, CLAUDE.md, README, Cargo.toml, ROADMAP, architecture, gaps, MCP, CLI, control, function parity and target parity reviewed. Important: parity documents contain stale claims; actual APIs and executable tests take precedence. Local plan/ is not distributed. craftrules was not accessible.

## Learning references
https://spreadsheetcenter.com/ — short tasks directly in cells with immediate feedback; guided tracks, no account required. Adopt the interaction mechanic, not wording or content.
https://blog.duolingo.com/learning-how-to-help-you-learn-introducing-birdbrain/ — models learner proficiency and item difficulty. We have no training dataset, so use documented bootstrap ranking.
https://blog.duolingo.com/spaced-repetition-for-learning/ — distribute review over time, mix prior material.
https://fmworldcup.com/about-us/ and official MEWC rules — contest cases and partial scores. Do not reuse contest assets. Exact identity of “Excel Wars” remains unresolved.
https://github.com/open-spaced-repetition/fsrs-rs — BSD-3-Clause, Rust scheduler/optimizer. Public API FSRS::next_states(previous_state, desired_retention, elapsed_days), returning per-rating states and intervals. Inspect pinned dependency before integration; procedural fluency/autonomy remain separate from memory state.

## AI references
https://ai.google.dev/gemini-api/docs
https://ai.google.dev/gemini-api/docs/pricing — official free tier has model/access limits. No assumption of unlimited access.
https://github.com/N1Moo/web2api — MIT; Worker/Docker gateway, compatible APIs, nonstreaming JSON validation, cookie-backed optional features. Its README's unlimited/free claims are not independently established; upstream web protocol can change. No gateway deployed and no credentials obtained.
https://github.com/HanaokaYuzu/Gemini-API — Python gemini_webapi, AGPL-3.0, uses account cookies and refresh behavior. No code copied; upstream clean-room rules exclude AGPL code.

Provider implementation and live tests are pending. Any later optional provider should have timeouts, bounded context, explicit privacy consent, redacted credentials and local fallback; never send credentials into browser builds.
