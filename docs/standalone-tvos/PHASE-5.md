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

## Failed-write retry correctness

Before exposing membership to Swift, a failure-injection test found a false-success
case: Core updates its library in memory even when encrypted persistence fails.
Repeating the same action suppresses the unchanged item update, clearing the
error without retrying disk storage. Red: `8da9ca6ecccb4400be2fd5438288a987`.
The test temporarily moves its own isolated storage directory, fails two attempts,
restores the directory and verifies a successful retry survives encrypted reopen.

Reasserting an already-matching membership now invokes Core's
`Internal::LibraryChanged(false)` persistence effect. It does not fabricate an
item modification or force a remote upload. All three library cases pass
(`2e1240a1009e42d583f6d06af0ab7275`); the complete native suite passes
(`72dee7cbb2844c639d93ca0e1346b9d4`), as do the 15 portable checks.
The FFI/Swift operation and acknowledged save state remain next; no UI currently
claims this operation has been durably saved. Remote retry remains Phase 5 sync work.
