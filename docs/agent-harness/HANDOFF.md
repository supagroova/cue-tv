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

Watched saves now cross C/Swift using an exclusive cancellable ticket and bounded
UTF-8 episode ID (zero length means film). Success waits for Core effects; accepted
mutations are not rolled back on cancellation. Receipt carries watched/video identity.
FFI invalid-input/busy/error checks pass (1afdc7b905a14377805a420c59fd871d;
missing-symbol red4ad09cd9dbaa479ebc2739ce6af5ac92). Swift missing API/state red:
e80a961a3ef640038ecb7d193f573239. Real HTTP Swift film watched/unwatched/stale
and held-effect cancellation checks pass with all60 tests
(0e2c7a4106f6443e86b6a31cc9bbe5b4). Full native suite passes
(72a81dd0ccee4d96965b76175f357f78), unsigned device build passes
(e0038d599b8e4cfe9dd1c40bacc1949e), 15portable checks pass.
Next: watched controls and remote UI journeys, including episode receipts and
failed-save/disappearance coverage. Manual-override/automatic playback precedence
and checkpoints remain pending. PR13 native/device CI passed; simulator still running
at last poll. Preserve root worktree's unrelated dirty CI docs.

Film/episode watched controls now pass remote journeys; season-navigation regression
was caught and fixed without weakening assertions. Evidence and remaining gates are
in PHASE-5.md. Unsigned device build and15portable checks pass. Next: Core Player
checkpoints/manual override plus outstanding UI failure/disappearance coverage.

Existing library journey also passes with watched controls
(7b3a5299aca74692a61c3b807d3da1e7). PR13 previous simulator run44174360266 is
still live; do not push over it without rechecking, to avoid wasting the run.
CI helper apple_ci_speed inspected read-only: seederbuild4m23, boot5s, combined
step started21:06UTC; live logs unavailable. No proven hang or failure.

Final title/video-scoped control identity passes the episode journey again
(d496899a1b444a12a7cc32e3f215c8b2) and unsigned device build
(90de6b049eee476e8759e106e92f622d). A rewritten PR13 body is prepared in
.agent-work/phase5-pr-update.json; apply it after the next authorized branch push.
Next playback sources are apple-tv/Sources/Playback/SourcePlaybackModel.swift
and SourcePlaybackView.swift; Core Player still throttles storage to180seconds.

PR13 watched/UI work pushed atab91481;14changed remote blobs verified and PR body
rewritten around actual behavior/remaining gates. Priorhead14b340b fullygreen:
run44174360266sim16m58/native5m48. CI helper found all3cachemisses: seeder4m23,
VLCKitresolve2m34,nativecrosscompile2m43,actualunit12s/UI2m16; caches saved.
Newhead Apple run44178385942: native134375797264running,sim134375797739queued
at21:20UTC. Recheck before superseding.

Next Core Player prerequisite now builds a private Player Selected from the exact
chosen stream/provider plus matching ready metadata request. Torrent/HTTPS prepare
share it and reject stale resource identity. Mixed metadata-stream/provider pages
retain source index and file7. Missinghelperred49f14ebc51984118bcc9a519d97e832c;
fullnativegreen c83150df66ef44d19dda1174747c3485; strengthened mixedpage testgreen
8c028f849e884833880757e50a42a4f8.60Swift tests pass
ba10886617cf459bb73324c81d8ace8e; unsignedbuild44adb4e8dfc242138bb23916bc62d363.
This constructs selection only: CoreSession does not yet own Player or receive
progress. Next retain admitted selection behind a bounded playback-generation
handoff, own Player/effects independently of catalog/detail generations, and bridge
Seek/TimeChanged/pause/end with acknowledged local checkpoints. Review automatic
addon event/intro-outro network behavior before enabling Player; Event messages are
currently dropped by session except fixed error categories, collect_seek_logs defaults
false. Do not mistake that for a complete no-telemetry audit of Player futures.

Player-effects audit found existing MetaDetails watched actions can dispatch
add-on library events; Player adds player events. NativeEnv now wraps add-on
transport and rejects library/player resource requests locally with a fixed policy
error. Content/manifest requests still use the upstream transport; no source URL
or viewing identity is logged. This does not block authenticated library sync.
Real HTTP regression red e306eec8d53a4caca744e55a21482773 proved an event was sent;
green b863325bb98244ac984f530a3007b81e confirms only manifest/catalog reach server.
Library fixture now advertises library/player: all mutation/persistence tests still
pass, proving policy errors do not turn accepted library saves into failures.
Fullnative5293b09412c64a9380c78a9af15f8306;60Swiftcfa4803098414699a0eb54a06978f28b;
unsignedbuilda1bd2b8f8e4d4172b17d2480a9f39cfb;15harnesschecks pass.
Remaining Player audit: direct rating WatchedSend and premium skip-gap futures use
profile auth independently of addon transport. Do not enable them implicitly when
integrating progress. Playercollect_seek_logs defaultsfalse; current session drops
Event messages except fixed error categories. Core Player integration remains next.
PR13ab91481native/device passed4m7; simulator134375797739 still queued at21:24UTC.

Prepared torrent/HTTPS receipts now include a nonzero playback_token. CoreSession
retains one private selected stream at most, replaces it on a new valid prepare,
and clears it on source/title reselection or logout. Tokens use a process-wide
checked atomic sequence. claim_playback consumes only the matching source-generation
selection once; invalid claims leave the current valid selection intact. This is
native handoff ownership only: Swift currently ignores the additive receipt field,
no C claim API exists, and Core Player/progress/checkpoints are still not connected.
Next: consume the retained selection inside a Core Player owner, bridge the token
into Swift callbacks, and separate local checkpoints from remote effects.

Real HTTP catalog test proves replacement rejection, single-use claims, exact
source/file identity and invalidation on title reselection. Missing API/field red:
6672f205db784b05907c4d6ceb15dc41. Catalog green: 3f02c89987e64ba28c405c8927798629.
Full native suite: e347418541074fc1b062c8cf419966e9. All 60 Swift tests:
b039912e01c040d8bc879b502f5eedf0. Unsigned device build:
a67bcc95982a4d7db8e3a66b144c5d7b. Fifteen harness tests pass.
PR13 remote remains ab91481; simulator job134375797739 remains queued, native/device
and Android passed. Recheck that live job before pushing the accumulated native work.

CoreSession now owns an optional token-bound Core Player accounting model
(native/cue-core/src/player.rs). start_playback consumes the prepared selection;
one active owner is allowed. playback_event accepts Seek/TimeChanged/Paused and
returns only position/duration/watched counters. retire_playback retires accounting;
it does NOT yet persist a close checkpoint. No C/Swift progress bridge is exposed.
Loaded Core metadata primes Player. Its returned effects are deliberately dropped:
explicit source/subtitle code owns retrieval, and future acknowledged local
checkpoints/sync must own persistence. Thus implicit Player rating, intro/outro,
stream-history and event futures are not executed. This is not a working resume UI.

Real-session tests prove seek adds no viewing time, strict >70% completion,
paused callbacks are ignored, backwards TimeChanged requires Seek, unknown duration
cannot complete, positions clamp to duration, replacement is busy until retirement,
and old tokens cannot mutate a replacement. Missing-API red:
2164048748af4c6abe8e6eefb48238cb. Full native suite passes:
f1b60a52e054472cae5619717cc62e7f. Seek-as-played mutation fails with800watched/true
instead of0/false (e9e6e8e84c0640c4ba57e2d3895edf34); restored catalog tests pass
(e651cc2491124e0d8bbf7106f9b857ed). Device build passes
48b403920b824edeb23dcd5737e10074 and15harness tests pass. Swift tests were not rerun
for this native-only, unexposed API; previous60 passed before this slice.
Next: manual watched override precedence and acknowledged local checkpoint pipeline,
then bridge token/events/lifecycle to the existing playback model. No automatic
network writes every5seconds; library checkpoint/journal must remain identity-scoped.
PR13ab91481sim134375797739 is now running, not queued; other checks pass.

Manual watched intent now reaches the matching active Player even when Core details
already has the requested value. It preserves the decoder baseline and prevents
later automatic completion for that owner. Film real-session regression red:
1dd89df5ccd34ac7aefbf1e5632386dd; green d9140c6412754ebcb573cfb031afde4e.
Series model tests prove unrelated title/episode changes are ignored and explicit
new Play re-enables Core completion without changing watched state during load.
A new regression found unchanged-position TimeChanged retained an obsolete duration
(ff31c3a2545e4eedad51b3cdf7359b78). Duration changes now use a no-viewing-credit Seek
baseline before played progress, including duration becoming unknown.
Full native suite passes87f8092af6e54403a548079f3e5171ed; device build passes
3b145ee096fa4ea29610795e4a7ecc48;15harness tests pass. No additional Swift tests
for this unexposed native accounting API. Next: acknowledged local checkpoints,
resume lookup and Swift token/progress/lifecycle bridge. Manual checkpoint projection
must preserve latest library membership and episode bits; do not overwrite a newer
manual edit with old Player state. Close retirement still has no durability barrier.
PR13ab91481 completed fully green: sim8m17, native/device4m7. Push the accumulated
native changes after this commit and verify remote blobs/current-head checks.

### Phase 5 atomic storage batch — local work in progress

`native/cue-core/src/environment.rs` adds a single encrypted replacement for a batch
of values/deletions. Missing-method red: 3768b258ba304ee7aca7097dbf7981aa; initial
green: 470d94a501c148ddba91d659cae6193e. Strengthened reopen/quota/update-delete
assertions and full native suite pass: aeaf8ac5ee654186a41931be0e221dce. Unsigned
device build passes: a088ce0eb1bd4753a940fd48a8a8a6a9; all 15 harness tests pass.
No checkpoint caller exists yet. Root worktree's unrelated doc edits remain untouched. PR13 at
70e9972 has passing native/device, Android and harness checks, simulator still
running at last observation. PR7 current head 2006470 has all six checks passing.

### Phase 5 checkpoint projection

`Playback::checkpoint_item` merges the owner's progress into current Core metadata,
membership and notification state, preserving other episode bits. Explicit completed
resume semantics survive late callbacks; unwatched remains false while progress
continues. The projector is pure and rejects a conflicting current title type.
Missing API red: c1efbfbb77de4eaca4c8a6d6a151b1cd. Episode green:
6218f3eaf6e9462a906edeff9239b011. Added film completion/unwatched and future-clock
coverage; full native suite passes 0fbe694bb3dd488db555c282eb417ce0; unsigned device
build passes 3c7d72d170f540788a5cb1c10d64ea68. No C/Swift consumer yet.
Next: session-owned acknowledged checkpoint queue using atomic recent/older storage;
serialize alongside existing sequential Core writes, without awaiting that queue
inside the actor (its completion messages need the actor to drain). Integrate journal
atomically before account release. Projection is not a durability receipt.
PR13 at70e9972 all checks pass; simulator9m05, native/device4m36. No open review
threads across current PRs at the latest audit. Full phase stack remains incomplete.

### Phase 5 explicit checkpoint receipt

Added `checkpoint.rs` atomic library serialization and session Checkpoint/CheckpointDone
commands. Canonical memory is updated on admission so subsequent Core library writes
snapshot current progress; the caller receives success only after queued encrypted
storage succeeds. One pending checkpoint maximum; concurrent save/retire return busy,
logout/login refuse while pending. Completion returns through the actor without
blocking its command drain. No network upload is scheduled by the checkpoint.

New `tests/checkpoint.rs` uses real Core models and a three-request owned HTTP fixture.
Missing API red b7b51639ba9f4b1d8e57b0fd50c4034b; initial test fixture wrongly called
source pagination and failed98ed74f64f644fe29051c8e83e7d7a67; corrected source load
passes68a5430454f240a8bc5b4cb604fb26a1. Extended held-write/busy/logout/progress-order
and encrypted failure/retry/reopen assertions pass in full native suite
28ed3bb084e14f89a34da07521d59244. Device build70691f02e7d94a6388ebde052c2c6009 passes.
Next: lifecycle barriers/coalescing, resume lookup and Swift callback receipt bridge;
atomic account journal remains required. Retirement still does not auto-checkpoint.

### Phase 5 native Close barrier

Added `close_playback`: shared checkpoint queue freezes progress during closing,
keeps the owner on failure and retires it on durable success before acknowledging.
Current accounting-only `retire_playback` remains explicit and makes no save claim.
Tests cover held close, duplicate close, late callbacks, failed close recovery and
encrypted reopen at600ms rather than the rejected late900ms. Missing API red
5458780f29574d7887128b83d8b50d9b; focused green c295546d1905411798835c0150aed6ed;
full native6c88a01a32b84c8ca535147951e49f27; unsigned build
fccf1a811d1c4fdb8405dd3ea157f0d0;15 harness tests pass. No Swift consumer yet.
Next: bridge native start/progress/checkpoint/close receipts and playback token into
Swift, then wire decoder callbacks with explicit seek classification and a bounded
periodic coalescer retaining lifecycle barriers. Resume and account journal remain
required. PR13 at18ca94a CI was still running at last observation.

### Phase 5 C/Swift playback receipts

C `cue_core_request_playback` uses existing take/discard ticket ownership and a
four-second native operation timeout. Swift validates action/token/progress shape
via the existing five-second reader. No new dependency. Integer ms input bounded
by INT64_MAX; unused time parameters must be zero. NativeCoreService exposes typed
actions and start/checkpoint/close wrappers; prepared source token is decoded.
Abort-only retirement deliberately makes no persistence claim. Cancellation may
leave an admitted owner/action in progress: the future UI owner must reconcile it.

FFI missing API red db2246eb083a4f85a92a2314bf90848c; green
4254cc12bc284c55970c7433d43b887a. Swift missing API red
071ff6a8cdb84bf6b83e789ece0db2b2. First complete Swift run passed the new bridge
test but existing initial account startup missed its1s polling budget
(fe43dc73498f4839acb4d445e98e53b5). Changed only that test's wait to an early-exit
3s deadline; all60 pass5989a7cbba86432f8ed739aedd6e2570. Full native suite
083ab8c7933d48fc9cbece1a512a3a13 passes (before that Swift-only test wait change).
Unsigned device build passes354f38d2acd84f92934f6dc884f8c6fb;15 harness tests pass.
Next: connect a single Swift playback accounting owner to decoder callbacks and
queued lifecycle barriers; keep seek/reposition separate from played progress.

### Phase 5 bounded Swift callback queue

Added PlaybackAccountingQueue: one asynchronous worker, bounded32 operations
including in-flight work, adjacent unsent played-sample coalescing, ordered explicit
barriers, sealed accepted Close, retained failure queue and explicit retry/flush.
Tests use a held external operation boundary, not mocked Core accounting. Missing
class red6a8575a2b99f499bbc50b52796f51d84; first62 Swift tests green
ce5120378e2a41faae6aaf636dea1dc0. Added capacity rejection/Close admission coverage;
all63 Swift tests pass008a7f5be1f247839f7f334a33e00373;15 harness tests pass.
Unsigned device build passes05519bd99ed74c8e8e9d3afd2f4ecdd8. Production UI integration
still outstanding. In particular, native timeout may have admitted an operation;
retry must not infer durable Close success from a stale token.

CI helper confirmed PR13 at18ca94a passed: simulator11m24, native/device3m49.
Native and Swift dependency caches hit, exact native artifact cache missed on
changed inputs. Slower compilation/UI execution explains most of the difference
from9m05; no failure or retry. Local checkpoint/Close/bridge commits remain to push.
