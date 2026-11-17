# Standalone tvOS handoff

The bootstrap notes below are historical. Current product progress is tracked in [implementation status](../standalone-tvos/IMPLEMENTATION-STATUS.md).

## Goal and state

Direct torrent playback on Apple TV; personal sideloading; Stremio-only v1. PRD, research and staged plan are in `docs/standalone-tvos/`. Source matches the extracted upstream baseline. No torrent engine, Stremio Core bridge or in-app player has been implemented. No physical-device playback is verified.

## Next work

Implementation stage 0: inspect Xcode/device compatibility, pair with the user's participation, configure a personal app identity and launch a debug Playback Lab. Then stage 1: embed and prove raw torrent transfer/playback before catalogue integration.

## Environment

A2737 / 64 GB / tvOS 27.2 is user-reported; verify OS/build during pairing. Local folder is a full checkout on `codex/tvos-agent-harness`, based on the saved planning branch. GitHub CLI authentication works over HTTPS using its credential helper; SSH authentication did not. The earlier extracted snapshot is retained in the sibling `cue-tv-snapshot-backup` directory. No signing configuration should enter source control.

## Evidence

Prior app baseline: unsigned build and five unit tests passed, as recorded in `docs/standalone-tvos/LOCAL-BUILD.md`. These do not prove networking/playback. Harness verification is recorded in `VERIFICATION.md`; rerun checks after subsequent edits. Device gates remain not run.

## Resume checklist

Read the relevant plan stage, inspect current files and branch when available, check task logs against current content, preserve completed work and pending authorization, then execute the next uncompleted task. Do not restart the investigation or re-ask confirmed product choices.

## Apple CI speed follow-up — 2026-11-17

Branch `codex/apple-ci-speed` is based on `614b74d`. The workflow now runs native/device and simulator jobs independently, preserves the aggregate check, and combines unit/short UI tests in one Xcode build. See [Apple CI execution](APPLE-CI.md) for exact commands, baseline timings, verification and limits. Local combined validation passed 25 unit and 10 short UI tests; the remote speedup is not yet measured. Next: integrate the reviewed commit, run the workflow and compare both cold and warm timings. Physical-device evidence is unchanged.


CI follow-up: warm role-cache run `44113416860` passed all checks in 9m02s queue-inclusive, with the simulator lane taking 8m52s. An ordering-only experiment now defers simulator boot until after cache/tool setup; local 12-test harness and actionlint pass, remote timing remains to be measured. See [Apple CI measurements](APPLE-CI.md).

CI artifact-cache follow-up: exact native-content simulator artifacts avoid repeating Cargo builds on matching inputs while preserving every test and device build. Local 15-test harness, workflow lint and shell syntax pass. Run 44123474614 passed cold and exact-hit attempts: warm simulator 7m32s, native/device 4m16s, queue-inclusive 7m47s; 31 unit and 10 UI tests preserved. The observed improvement over earlier warm simulator runs is 80–100s, with runner variability. See APPLE-CI.md for scope and invalidation rules.

CI cache routing follow-up: simulator final-artifact hits now skip unused Cargo setup while keeping a separate Swift package cache. All15 portable tests and actionlint pass; remote timing still pending. See APPLE-CI.md.


Phase 5 starts on `codex/tvos-phase-5`, based on Phase 4 `0bbecfa`. The state/sync
contract is in `docs/standalone-tvos/decisions/003-resume-library-sync.md`; implementation
status and next slice are in `docs/standalone-tvos/PHASE-5.md`. This is a separate
worktree under `.agent-work/phase5`; the root worktree remains Phase 4 with its two
unrelated unstaged documentation additions preserved. No Phase 5 runtime claim.

Phase 5 native membership actions now pass the full Core suite (record
`ff81794c3261435dbdb4d1793429466d` in this worktree's phase-5-library task).
Add/remove encrypted reopen tests pass; next is FFI/Swift integration and truthful
save acknowledgement. Root Phase 4 remains untouched. PR7 is now all green.

Membership retry now re-persists unchanged Core state after disk failure. Full
native suite passes (`72dee7cbb2844c639d93ca0e1346b9d4`). FFI/Swift save receipt is
still pending. PR8 Android Build run44168259064 failed on Maven HTTP429 and its
failed job was rerun; Apple TV jobs were left intact.
