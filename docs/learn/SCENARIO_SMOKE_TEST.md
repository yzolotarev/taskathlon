# Procurement route: desktop validation checklist

This is a manual gate, not a record of a completed usability study.

The route is tested against the adopted ladder model (micro-actions to a
composite brief). Retention scheduling (FSRS) and gradual difficulty scaffolding
(levels 1..4 with fading) exist; the explicit atom/skill dependency graph is
deferred. The checks below distinguish result checks, operation checks and
transfer evidence, and do not claim desktop Excel parity.

## Before testing

Use a separate test profile; keep the learner's progress unchanged. Run the
Taskathlon binary and record operating system, keyboard layout, window size and
whether the desktop is real or rendered offscreen.

## First session

- The procurement route has an obvious entry point.
- Typing starts in the intended cell without a mouse click.
- Enter commits one value once; the same key event cannot skip another step.
- A wrong answer remains editable without a blocking dialog.
- Assistance is marked; using it does not establish independent mastery.
- The learner can select a range with Shift and use Ctrl+D.
- Filled formulas reference the expected rows, rather than copying cached numbers.
- Correct micro-actions give local feedback, without mandatory Next banners.
- Large/small windows keep the table and instruction legible.
- Pause suspends the route's clock; resuming preserves work and focus.

## Ladder variation (seeded)

- The same skill appears at different cell positions across attempts; a retry
  does not reuse the previous layout.
- The numbers differ between attempts while the required method stays the same.
- A layout is stable within a single attempt, so the learner is not chasing a
  moving target mid-step.
- A hard-coded constant is rejected by the altered-data probe: when the input
  data changes, a baked-in answer no longer matches and the step fails.

## Corrective insertion

- A wrong answer opens corrective practice inside the same flow, not a separate
  session or a blocking restart.
- Returning from correction preserves the learner's saved work; the correction
  does not overwrite or discard completed steps.
- After correction the learner resumes at the interrupted step, not at the
  beginning of the route.
- Corrected work is recorded as assisted/corrective evidence, not as a clean
  independent success.

## Work request

- The learner independently calculates five order lines and the total.
- Source quantities can be edited when explicitly requested.
- Initial order total is 236000 KZT; increasing cable quantity from 30 to 45
  changes it to 254000 KZT (+18000), without replacing formulas manually.
- A total typed as a constant (e.g. 236000) rather than a formula is rejected by
  the altered-data probe when the source data changes.
- Incorrect or incomplete work cannot be silently accepted.
- The final case requires an explicit submission, not an accidental transition.

## Restart

- Close on an introductory step and resume the same work.
- Close after a correction and resume without duplicate rewards.
- Close after a successful submission and do not resubmit the same evidence.
- Corrupt or incompatible scenario state is reported and preserved, not mistaken
  for a successfully completed route.

## Advanced route

- Lookup includes exact match, fixed ranges and unknown articles.
- Supplier totals and status totals agree with independent calculations.
- A real pivot is created through the UI; typed totals alone do not pass the
  pivot-operation criterion.
- Updating source data and refreshing the pivot changes the report correctly.

## Learning transfer

Give a fresh layout without naming the required function. Record accuracy,
assistance, elapsed time and the learner's explanation separately. Repeat after
a break. If Microsoft Excel is available, verify key operations there too.
Neither an automated UI pass nor this checklist establishes a hiring guarantee.
