# Optimising the existing learning loop

This iteration improves existing drills, remediation, FSRS and sprints without an external AI service.

- **Action diagnosis:** first submissions distinguish an exact reversed strict comparator from an incomplete formula column. Relative-reference copying is checked independently of formula correctness; Ctrl+D credit requires both the shortcut and correct copying. Ambiguous formulas keep normal grading feedback. Action evidence persists and appears in progress.
- **Dependencies and reviews:** the remediation planner consults a small prerequisite map only when prior failures support returning to that prerequisite. It preserves the original target and all unconsumed remediation. Cases review a skill once per submission only when every criterion for that skill passes.
- **Accuracy before speed:** ordinary lessons hide the stopwatch. Speed cannot skip the copying stage, double mastery progress or stretch FSRS intervals. Explicit test-outs still skip ahead. Sprints require three recent unaided first-attempt successes at level 2 or higher, with checks at both the UI and start handler.

Regression fixes preserve graded feedback, first-attempt status and method choices across restart. Opening another workbook cannot submit it as the exercise, and saving preserves the actual exercise book. The native host now supplies Open/Save As pickers.

Diagnosis currently covers strict reversed comparisons and formula-column completion. It deliberately does not infer arbitrary conceptual mistakes from the final score. The dependency map covers existing logic, conditional aggregation and reference skills; it is not a complete curriculum graph.
