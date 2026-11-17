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

Phase 5 C save receipt is implemented; five FFI tests pass in record
`bcbf7ccfd4ee430589e43eec24336ead`. Next: Swift service and real HTTP fixture
success/failure receipt tests before UI controls. Local compiler output exhausted
disk space; Cargo clean -p cue-core freed 11.9 GiB, preserving test evidence.

Swift membership receipts now pass the real HTTP fixture and all 60 Swift tests
(`7819d142aaba44e48f7621119a44c4f7`); unsigned device build passes
(`8ab87378acaf48be88a478f356765a00`, docs-only evidence edit during build).
Next: guarded UI library controls and interrupted save coverage. Phase5 uses
symlinks to the root native-target/DerivedData caches; never build concurrently
in those worktrees. PR8 Android retry completed successfully.

Title-detail library controls pass the remote add/remove/source/Back journey
(`b8b5344c99d24adaafdb2c819652f8b8`) and unsigned build
(`1b8bf4c72fb84d81a6e575e1d302c58b`). Next: interrupted/failed receipt UI coverage,
then remaining Phase5 library browsing/watched/player checkpoint work. PR8 native
job134340042364 was retried after a proven runner-capacity cancellation; PR13
simulator134341747698 likewise never acquired a runner. PR13native remains live,
so local Phase5 commits have not yet been pushed over that run.

Pending library receipt cancellation is covered with a held HTTP response and
sensitive mutation test. Restored source passes all 60 Swift tests
(`e20236255b054007845c0b862068b558`). Phase5 runtime changes are ready for the
next PR13 push; its previous run is terminal (nativepass, simulatorcapacitycancel).
Next implementation: bounded library browsing, watched controls and Core Player
checkpoints, while retaining the documented UI failure/disappearance test gaps.

Native library browsing now has 24-item windows and revision-guarded selection;
full Core suite passes (`4cb6534b19ef4ab19804f8db1ea7d69d`). Next: expose
show_library/load_library_details through FFI and Swift, then My library UI with
origin-aware Back focus and an owned add→browse→reopen→remove journey.

My library now browses Core pages and reopens saved titles through metadata;
remote add→browse→reopen→remove passes (`52507771e08e46248251a96b6ddf57a1`),
60 Swift tests and device build pass. PR13's CI collision is fixed with an atomic
temporary-directory suffix, preserving parallel execution. Next: watched controls
and Core Player checkpoints, plus remaining UI failure/lifecycle coverage.

Watched-state detail projection now reads Core film completion and per-video
bitfields, with matching title/type guards. Episode 30 retains its watched state
across paging; a film resume position alone does not imply completion. Missing
fields reproduced red (`204e1f28ca9e44d695eadc5d10c6d570`), detail tests pass
(`d5af79fea77f445cb5850bb18b1aa16a`) and full native suite passes
(`16b3fc21d5ba411fb100490d23bf74ef`). No watched mutation/UI/checkpoint claim yet.
Next: revision-guarded Core watched actions, durable receipts and reopen tests,
then Swift controls and manual-override integration with Core Player.
PR13 Apple run44174360266 jobs134361659661/134361661187 were queued with no
runner at 21:00 UTC; Android and portable checks passed. PR7 remains fully green.

CoreSession now admits revision-guarded film/episode watched changes using Core
MetaDetails actions. Invalid video identities and conflicting library types are
rejected; retries flush storage without incrementing a film's completed count.
Owned HTTP tests cover both watched values, unchanged membership, separate episode
bits, two failed writes followed by recovery, disk contents before any membership
write, and encrypted reopen. Missing API red: 72a58da8bc504293a2fb09684ba5c3ac.
Removing idempotence fails with count4 instead of1 (06778f90da2b44b5898af63bbb078ba3).
Restored full native suite passes (5fdcd20735474fb8b989e4f8a0a266c1); additional
pre-membership disk assertions pass (a6583a2088f34191a59a5a0e31f7430a).
Next: bounded cancellable FFI watched receipts, Swift/UI, then Core Player manual
precedence and checkpoints. These session actions acknowledge admission only;
settled/error must be inspected before reporting durable success. Remote sync
retry remains unimplemented. PR13 Apple jobs are now executing; no review threads
were present on open stack PRs at 21:03 UTC. Local watched work is not yet pushed.
