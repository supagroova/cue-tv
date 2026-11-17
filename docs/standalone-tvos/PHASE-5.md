# Phase 5 — resume, library and recovery

Status: implementation started with the required state/sync decision record.
Runtime integration, lifecycle tests and user-facing controls are not implemented
by this initial change. Earlier phases retain their outstanding gates.

The contract and ordered implementation gates are in
[decision 003](decisions/003-resume-library-sync.md). Source inspection found a
180-second Core progress-write throttle and whole-second sync comparisons;
relying on those unchanged would leave resume and conflict requirements unproven.

Branch: `codex/tvos-phase-5`, stacked on `codex/tvos-phase-4` at `0bbecfa`.
Next: implement Core-backed library actions with revision guards and persistence
reopen tests, then integrate Core Player and acknowledged checkpoints. Device
qualification is deferred until the Apple TV is available; simulator work continues.
