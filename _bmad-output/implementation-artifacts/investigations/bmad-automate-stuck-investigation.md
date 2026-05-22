# Investigation: bmad-automate appears stuck

## Hand-off Brief

1. **What happened.** `bmad-automate.sh` resumed Story 6.2 in review, skipped create/dev, then waited inside Claude-driven test architecture steps with little or no terminal output.
2. **Where the case stands.** Root cause is confirmed: the automation treats `PASS_WITH_ADVISORY` as failure even though the trace step reports full testable-layer coverage and zero release blockers.
3. **What's needed next.** Decide whether `PASS_WITH_ADVISORY` should be accepted as terminal for automation; if yes, relax the gate check and add clearer progress output around silent Claude steps.

## Case Info

| Field | Value |
| --- | --- |
| Ticket | N/A |
| Date opened | 2026-05-22 |
| Status | Concluded |
| System | macOS 26.5, zsh, Codex desktop, `bmad-automate.sh` |
| Evidence sources | `bmad-automate.sh`, sprint status, current and prior automation logs, trace gate decision |

## Problem Statement

User reported `./bmad-automate.sh` appeared stuck after:

- `Continuing review-stage story: 6-2-real-time-dashboard-updates`
- `Skipping create-story and dev-story because 6-2-real-time-dashboard-updates is already in review.`

## Evidence Inventory

| Source | Status | Notes |
| --- | --- | --- |
| `bmad-automate.sh` | Available | Control flow and wait behavior inspected. |
| `_bmad-output/implementation-artifacts/sprint-status.yaml` | Available | Story 6.2 is in `review`, so the script correctly resumes it. |
| `_bmad-output/test-artifacts/traceability/6-2-gate-decision.json` | Available | Gate is `PASS_WITH_ADVISORY`, not exact `PASS`. |
| `_bmad-output/automation-runs/20260521-222606/testarch-trace-6-2-real-time-dashboard-updates.log` | Available | Prior trace step failed on Claude session limit. |
| `_bmad-output/automation-runs/20260522-052255/checkpoint` | Available | Current run was in `testarch-trace` at the time of investigation. |
| Claude internal session state | Partial | Current process was alive and later completed; `.claude` JSONL showed activity while terminal logs were silent. |

## Investigation Backlog

| # | Path to Explore | Priority | Status | Notes |
| - | --- | --- | --- | --- |
| 1 | Confirm whether `PASS_WITH_ADVISORY` should skip trace regeneration | High | Open | Product/process decision. |
| 2 | Add no-output heartbeat or timeout around `run_claude` | Medium | Open | Engineering hardening. |
| 3 | Add Claude quota/session-limit preflight | Medium | Open | Current auth preflight only proves the command can answer a tiny prompt. |

## Timeline of Events

| Time | Event | Source | Confidence |
| --- | --- | --- | --- |
| 2026-05-21 22:56 +07 | Previous Story 6.2 `testarch-trace` failed because Claude hit session limit. | `_bmad-output/automation-runs/20260521-222606/testarch-trace-6-2-real-time-dashboard-updates.log:1` | Confirmed |
| 2026-05-22 05:23 +07 | Current run resumed Story 6.2 in review and skipped create/dev. | `_bmad-output/automation-runs/20260522-052255/master.log:20` | Confirmed |
| 2026-05-22 05:28 +07 | Current run completed `testarch-automate` run 2 and entered `testarch-trace`. | `_bmad-output/automation-runs/20260522-052255/master.log:22` and `_bmad-output/automation-runs/20260522-052255/checkpoint:1` | Confirmed |
| 2026-05-22 05:34 +07 | Current `testarch-trace` completed with `PASS_WITH_ADVISORY`, then script exited because `verify_trace_gate` expected exact `PASS`. | `_bmad-output/automation-runs/20260522-052255/master.log:41` and `_bmad-output/automation-runs/20260522-052255/master.log:73` | Confirmed |

## Confirmed Findings

### Finding 1: Story 6.2 is in review, so create/dev skip is expected

**Evidence:** `_bmad-output/implementation-artifacts/sprint-status.yaml:89`; `bmad-automate.sh:675`; `bmad-automate.sh:700`

**Detail:** The script chooses `review_story` first, assigns it to `story`, and prints the skip message when the story already equals `review_story`.

### Finding 2: Existing trace gate does not satisfy the script's skip condition

**Evidence:** `_bmad-output/test-artifacts/traceability/6-2-gate-decision.json:12`; `bmad-automate.sh:311`; `bmad-automate.sh:704`

**Detail:** The gate status is `PASS_WITH_ADVISORY`. `has_trace_pass` only returns true for exact string `PASS`, so `SKIP_EXISTING_TRACE_PASS=1` does not skip the test architecture steps.

### Finding 3: Claude steps can produce no step log until the child exits

**Evidence:** `bmad-automate.sh:433`; `bmad-automate.sh:437`; `bmad-automate.sh:377`; `_bmad-output/automation-runs/20260522-052255/master.log:22`

**Detail:** `run_claude` runs `claude -p ... > >(tee "$log")` and `wait_for_logged_tool` only checks for auth-error strings while the process is alive. During the current investigation, `testarch-trace` remained zero bytes while the child process stayed alive.

### Finding 4: The previous run did not complete because Claude hit a session limit

**Evidence:** `_bmad-output/automation-runs/20260521-222606/testarch-trace-6-2-real-time-dashboard-updates.log:1`

**Detail:** The prior trace log contains `You've hit your session limit · resets 1:40am (Asia/Jakarta)`, after which the script reported Claude exit code 1 in `master.log`.

### Finding 5: The current run completed trace generation but failed the gate check

**Evidence:** `_bmad-output/automation-runs/20260522-052255/testarch-trace-6-2-real-time-dashboard-updates.log:4`; `_bmad-output/automation-runs/20260522-052255/testarch-trace-6-2-real-time-dashboard-updates.log:17`; `_bmad-output/automation-runs/20260522-052255/master.log:73`

**Detail:** The trace step reported `PASS_WITH_ADVISORY`, 100% P0/P1/overall coverage, all 3 ACs full-covered at the testable layer, 4 advisory gaps deferred to manual browser verification, and 0 release blockers. The shell script then rejected that status because `TRACE_GATE_CHECK=require-pass` expects exact `PASS`.

## Deduced Conclusions

### Deduction 1: The apparent stuck point is not the skip message itself

**Based on:** Findings 1, 2, and current checkpoint.

**Reasoning:** The script printed the skip message, then continued into the trace gate branch because no exact `PASS` existed. The checkpoint later showed `stage=testarch-trace`.

**Conclusion:** The visible hang is after the skip message, inside a Claude test architecture step.

### Deduction 2: The script re-runs expensive Claude steps even when the story has a non-failing advisory gate

**Based on:** Finding 2.

**Reasoning:** `PASS_WITH_ADVISORY` is a non-failing advisory outcome, but the script treats it the same as missing/failing for skip purposes.

**Conclusion:** If advisory gates are acceptable for this automation, the skip condition is too strict.

### Deduction 3: The script's auth preflight cannot detect the prior failure mode

**Based on:** Finding 4 and `bmad-automate.sh:459`.

**Reasoning:** The preflight asks Claude for a tiny `AUTH_OK`; the failing step is a large `bmad-testarch-trace` run. Session/quota exhaustion can occur after the tiny preflight succeeds.

**Conclusion:** The preflight verifies auth, not availability of enough Claude session budget for the automation.

### Deduction 4: The actual current exit condition is gate-policy mismatch

**Based on:** Findings 2 and 5.

**Reasoning:** The trace agent says the story has no release blockers, but the shell gate admits only exact `PASS`.

**Conclusion:** Current automation stops because shell policy and BMad trace policy disagree on whether advisory-only gaps are terminal.

## Hypothesized Paths

### Hypothesis 1: Current run may eventually fail with the same Claude session-limit message

**Status:** Refuted

**Theory:** The current `testarch-trace` process is a large Claude request similar to the prior failing step, so it may terminate with the same session-limit error.

**Supporting indicators:** The previous trace step failed exactly this way; the current trace log stayed zero bytes for minutes.

**Would confirm:** Current `testarch-trace` log gets the same session-limit message.

**Would refute:** Current `testarch-trace` completes and writes refreshed trace artifacts.

**Resolution:** Refuted by current run completion: `testarch-trace` produced refreshed artifacts and exited with `PASS_WITH_ADVISORY`; the shell then failed at `verify_trace_gate`.

## Missing Evidence

| Gap | Impact | How to Obtain |
| --- | --- | --- |
| Whether the organization wants `PASS_WITH_ADVISORY` to be terminal for automation | Determines whether to change `has_trace_pass` | Human process decision. |
| Exact live Claude API state during silent period | Distinguishes long inference from server-side stall | Wait for current process completion or inspect Claude CLI diagnostics if available. |

## Source Code Trace

| Element | Detail |
| --- | --- |
| Error origin | `bmad-automate.sh:311`, exact `PASS` check in `has_trace_pass` |
| Trigger | Story 6.2 status `review` plus existing gate status `PASS_WITH_ADVISORY` |
| Condition | `SKIP_EXISTING_TRACE_PASS=1` does not skip because `PASS_WITH_ADVISORY != PASS` |
| Related files | `_bmad-output/test-artifacts/traceability/6-2-gate-decision.json`, automation run logs |

## Conclusion

**Confidence:** High

The script is not stuck at the printed skip line. It proceeds into `testarch-automate`/`testarch-trace` because the existing gate is `PASS_WITH_ADVISORY`, while the script only skips on exact `PASS`. The current run completed trace generation and then exited because `verify_trace_gate` rejects `PASS_WITH_ADVISORY` under `TRACE_GATE_CHECK=require-pass`.

## Recommended Next Steps

### Fix direction

If `PASS_WITH_ADVISORY` should count as good enough for automation resume, update both `has_trace_pass` and `verify_trace_gate` to accept it when `critical_open == 0`, or rename the functions to reflect accepted terminal gate statuses.

Add a heartbeat/no-output timeout to `wait_for_logged_tool` so the terminal prints the active stage and log path while Claude is silent. Consider a stronger Claude quota/session preflight or a clear retry/backoff path for `session limit` messages.

### Diagnostic

Rerun with `TRACE_GATE_CHECK=warn ./bmad-automate.sh` if the goal is to continue past advisory-only gates without code changes. For a permanent fix, encode the accepted advisory status in the shell gate policy.

## Reproduction Plan

1. Keep Story 6.2 in `review`.
2. Keep `_bmad-output/test-artifacts/traceability/6-2-gate-decision.json` at `gate_status: PASS_WITH_ADVISORY`.
3. Run `./bmad-automate.sh`.
4. Observe that it skips create/dev, then enters `testarch-automate` and `testarch-trace` instead of skipping existing trace artifacts.

## Side Findings

- The BMad investigate resolver requires Python 3.11+; system `python3` is 3.9.6, but Codex bundled Python 3.12.13 works for the resolver.
