# Learning from spreadsheet errors

During ordinary spreadsheet drills, committed errors in answer cells trigger a short
mentor explanation on their first encounter. Detection waits until editing has finished;
errors deliberately supplied by the exercise are not treated as learner mistakes.

The next task asks the learner to identify what to inspect, then repair a calculation
on seeded data. This teaches the cause before introducing error suppression. In
particular, returning zero with IFERROR is not presented as a universal solution.
The current catalog covers the engine's eleven error values, including circular
references. Examples are displayed as text so they cannot corrupt the practice workbook.
The SPILL exercise asks for a scalar result; it does not simulate clearing a spill range.

First encounters and pending practice persist in the learner profile. Skipping or
quitting retains the pending exercise; passing removes it. Completed error skills use
the existing FSRS cards for later reviews. Mentor assistance counts as a hint and is
disabled for sprints, checkpoints and test-outs, including after a restart.

The mentor introduces an error category, not a definitive diagnosis of every formula.
For NAME, for example, an unsupported engine function is also possible. A new category
does not lower an unrelated skill merely because its explanation was shown.

## Verification

Regression tests cover first encounters, profile round trips, all repair references,
skipping and completing follow-up work, and assessment isolation. The lesson UI uses
the existing mascot; no external AI service is required.
