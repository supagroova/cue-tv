# Phase 5 — resume, library and recovery

Status: native library membership actions implemented; Swift/FFI integration,
watched controls, playback checkpoints, sync and lifecycle work remain incomplete.
Earlier phases retain their outstanding gates.

The contract and ordered implementation gates are in
[decision 003](decisions/003-resume-library-sync.md). Source inspection found a
180-second Core progress-write throttle and whole-second sync comparisons;
relying on those unchanged would leave resume and conflict requirements unproven.

Branch: `codex/tvos-phase-5`, stacked on `codex/tvos-phase-4` at `0bbecfa`.
Next: implement Core-backed library actions with revision guards and persistence
reopen tests, then integrate Core Player and acknowledged checkpoints. Device
qualification is deferred until the Apple TV is available; simulator work continues.

## Native membership slice

CoreSession accepts add/remove only for the selected, successfully loaded metadata
and current revision. A conflicting type under Core's ID-keyed library is rejected.
Core actions own the mutation and storage effects; snapshots expose only an optional
selected-title membership flag. Admission does not mean durable save: consumers
must await settled state and handle its persistence error before reporting success.

The real HTTP fixture exercises metadata selection, stale revision rejection and
encrypted reopen for both addition and removal tombstones. It confirms plaintext
title identity is absent from the ciphertext. Missing-API red:
`720688d373a64702bbfb7c6033ac5734`; first add/reopen green:
`1e210d270b044d9c942a7da4cd3029f1`; full native suite with both cases:
`ff81794c3261435dbdb4d1793429466d`. All 15 portable tests pass. No Swift/FFI
command is exposed yet, no Apple build was run for this native-only slice, and
account synchronization/manual watched policy is not established by these tests.

Next: expose the operation through FFI and the Swift service, with an acknowledged
save state, then add user-facing library controls and continue decision 003.
