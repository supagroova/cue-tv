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

### Phase 5 decoder accounting adapter — local verification

Added PlaybackAccountingOwner and PlaybackReposition. Explicit decoder activation
rejects stale sample generations. It classifies initial/resumed/backward/reposition
clocks as Seek, ordinary playback as played, pause as seek+pause+checkpoint, and
uses a five-second monotonic deadline. Reposition samples wait within500ms of the
clamped target; actual VLC callback integration still needs validation. Callback
groups use atomic queue admission; capacity cannot leave half a seek/save operation.
Owner close drains queued work and retained rejected work before durable Close.

Missing-class red9686db785a8947b399c915f0fdcf6e9d; initial64 tests green
039a93045ccd44da956309d10e9e6331. Stale-generation regression reproduced
6b767993f3d24818a0cad34e45af9ef3 (late callback added unwanted unpause/seek/save).
Explicit activation fix and partial-batch test: all65 Swift tests green
e0b21d6ee8ba49d8a772fef6d9c15d90. Device build passes
85332a6854d2452bba6acd1f91024fb0;15 harness tests pass. No native changes in this slice.

UI integration findings: SourcePlaybackView polls VLC at100ms, tears transfer down
immediately on Close and currently lacks duration/reposition metadata. Add duration
and seek markers in VLCPlaybackSurface, including caption-refresh repositioning.
Retain accounting owner in StandaloneModel (not only view State): a disappeared
view whose close fails must not lose the token and leave the next source permanently
busy. Before new prepare, close the old owner and revalidate selection/revision.
Source retry should explicitly activate a new decoder generation on the same Core
owner. Avoid implicit generation switching from callbacks. Concurrent start calls
and async view disappearance need tests before wiring; current start reports false
while another start is pending. Resume choice/initial-position retrieval must precede
resetting an existing resume offset with a new decoder's zero position. Existing
C/Swift operation cancellation does not undo already admitted native actions.

### Phase 5 VLC timing and concurrent startup

Fixed the previously documented concurrent-start race. Held Start reproduced the
second caller's false result332ffc3e9b7c47b2aa73e062c466e090. Both callers now await
the one queued Start; first green b8e340a2ae2e416fb6caa3580098d728. Added Close
during Start and explicit not-finished-before-release assertions. All67 Swift tests
pass8f695640ef5a423d9188fc5cea013baf; final three targeted owner tests pass
3caa0e3223584656a119e5ca5374451b. The first targeted invocation omitted pinned
Cargo environment and failed build d6a9446d187a44e5963155a3102eacf7; rerun included
the documented CARGO_HOME/RUSTUP_HOME/PATH. This was not an application failure.

VLCPlaybackSnapshot now carries durationMilliseconds and PlaybackReposition.
Explicit remote seek uses its request UUID; subtitle-refresh reposition gets a new
UUID. SourcePlaybackView publishes duration as the elapsed clock's accessibility
value. New36-second duration assertion failed before implementation
b3170b6731274aa7a988b9454db91d27 and passes the real owned peer journey
e4c1dfb66d18483fa1849b89ee309734 (decoded green frame and Close also retained).
No Core accounting callback wiring yet. Unsigned device build passes
e3427e8cb80b49a9bf1a26b45edcd2b7;15 harness tests pass.
Next still requires retained parent accounting ownership, generation activation,
resume-position retrieval/choice before decoder starts, then actual callback/lifecycle
integration. A service pause receipt can inspect the newly started Core position
without adding watched time; verify same-video versus different-episode restoration
before using it for Resume. No library journal or physical qualification completed.

### Phase 5 resume identity restoration

Regression aba02794a5694e3a8c805db16dce1b6b proved a newly selected episode retained
the preceding episode's1000ms duration. Native Playback::new now checks the stored
video ID before retaining position/duration; different identity resets those fields
to0 without changing the library. Same-video901ms/1000ms/901ms viewed survives
initialization. Full native suite passes e842ee5577ab43aab6ca416fefbf5c95.

Extended tests/checkpoint.rs to keep the owned HTTP fixture across encrypted reopen,
serve a different torrent hash on second resolution and create a fresh Core session.
New token and source are verified; paused progress restores600ms/1000ms/600ms viewed,
unwatched. Targeted integration passes e51c1d73d8f446c2ad4cdd301cf50976. No additional
production change followed the full native run. Unsigned device build passes
5217c8bf49284cdc95a92d9d55fe299b;15 harness tests pass. No decoder/UI resume claim.

Important UI sequencing: starting Core to inspect Resume is not yet decoder playback.
If the user cancels before any decoder observation is admitted, retire without saving
rather than checkpointing a new episode's zero state over the old continue-watching
pointer. PlaybackAccountingOwner now retires without saving if no decoder operations were
admitted. Decoder activation, invalid negative timing and non-playing states alone
do not authorize saving. Successfully replayed backpressure intent counts as an
observation; normal playback still ends with durable Close. Repeated Close remains
single-use. The close-during-start test intentionally now expects retire rather
than save. Red: 1ae779f808c3476abbc2293f8a587856 (three expectations expose unwanted
save). Green: 82760ca47fb24a3eb96d660f8062fc46, all four accounting owner tests,
including the existing seek/pause/periodic-save/durable-close ordering test.
Fifteen portable harness tests pass. Full simulator suite now passes all 68 tests (30bcc6708e4b4850917ae1ad3b7ff703);
unsigned tvOS device build passes (35750d6679c04c75a4e3d898276fafb4). Parent retention and resume choice still
required as described above.


### Phase 5 — buffered observation retry ownership

A blocked native operation can fill the bounded accounting queue. Close admits the
retained decoder observation after draining. If execution then fails, the queue
already retains it; keeping a second copy in PlaybackAccountingOwner caused a
second successful seek/checkpoint after retry. Transfer ownership at admission,
before awaiting the result. Regression fills the real queue using distinct seeks,
fails the retained seek once, then verifies exactly two attempts (failure + retry)
and one durable Close. Red fa99e8d747c84f26a86cdcd267b41cee failed only the attempt
count. Green 4d7e495960b4499ca2d0b352bd3fde31 passes all five accounting owner tests.
This does not resolve ambiguous native timeouts; production integration still needs
receipt reconciliation, parent retention and Resume/Restart choice.


### Phase 5 — initial resume receipt before decoder admission

PlaybackAccountingOwner now accepts typed Core progress receipts. Startup atomically
queues Start then Pause; Core Pause exposes restored position without adding viewed
time or persisting. The first receipt is retained separately from later decoder
pauses and published as initialProgress only after the queue drains and Close has
not begun. Decoder activation is blocked until that point. Missing initial progress
fails startup. Close while Start or initial Pause is pending drains and retires,
without publishing a Resume choice or saving the previous record.

Intentional ordering change: startup adds Pause before any decoder Unpause/Seek.
Red ddf6f8a6965644b5a98b9f023d626ae7 verifies missing Pause in cancellation and normal
playback sequences. Initial five tests pass 33f31d2197ef499eaa36dc961a47e43d. Added
held-Pause/Close and missing-receipt coverage; full simulator suite passes 71 tests
c3bd0ef3fe1d47478042628ce8d1f7d1. Unsigned tvOS build passes
ef7ace8aaf704167b58d8093d5118802. The execution closure can now return service.playback's
typed result directly; production StandaloneModel retention, Resume/Restart UI and
VLC wiring remain the next integration task. No physical-device claim.


### Phase 5 — parent-owned accounting lifetime

StandaloneModel owns PlaybackAccountingSession and exposes accounting(for:) plus
closePlayback(). The registry returns the same owner for a repeated token, serializes
replacement/close, and retains failed saves. A replacement source, Core replacement,
and polling shutdown/startup now await accounting retirement. On failure the Core
service is retained, with a retry message, instead of silently dropping the save.
Source preparation rechecks Core's authoritative revision after retirement; a changed
source list requires reselection. CorePlaybackRequest exposes its native token.
The full-screen player does not yet call accounting(for:); wire it next, including
Resume/Restart, failure UI and its disappearance path.

Storage-fault test checks failed Close, blocked replacement, same-owner retention,
and exactly one successful durable Close before replacement. Sensitivity run
44168d5649be42bdb40dc8ec00c9276f deliberately discarded the failed Close and failed
four assertions. Initial green 1bbc93d35bf740c18557b84d93dfb3b9. A second test exposed
a real late-view-task race: an acquired but unstarted owner could start native work
after replacement had retired it (59be243520144343abfc540df451d765). start() now
rejects closing owners before enqueueing. Full suite passes 73 tests in six suites
c8f049839e1842218fdd050665460d0d; unsigned device build passes
9c206c61e9f9496db1c0346e3a363e95. No device playback or production UI integration claim.


### Phase 5 — player screen integration (18 November)

StandaloneHome now supplies the parent-owned accounting session to SourcePlaybackView.
Core Start/Pause completes before transfer/decoder startup. A saved position offers
Resume/Start from beginning; Close at this choice retires without saving. VLC timing,
duration, pause and reposition markers feed the generation-guarded accounting owner,
which applies its five-second cadence. Close/Back/deactivation stop resources and
await accounting Close; failed saves expose retry and remain parent-owned. Unexpected
view disappearance also attempts retirement. The debug standalone HTTPS fixture has
no Core selection and intentionally omits this provider.

Opt-in peer journey initially failed the missing Resume/Restart and decoder-before-
choice assertions (ce244638b26c479988a70d93eec26449). Integration passed
c46370960afe4edf9e6ca120298285f3. Extended journey plays at least eight seconds,
closes, opens and cancels Resume, reopens with the identical saved label, and resumes
to within one second of the saved position within five seconds of decoder display.
Its first run d33414e8bac0492a89fd256a347e905d passed resume assertions but sent Back
before dismissal restored focus. Added the same source-focus wait used by the first
Close; full journey passes 5457ab4885f34286b71c1dd558d91bce. Subtitle on/off/reselection
journey passes 641a49e45f1346dea337611a2d9a40f3. These remain opt-in; routine CI still
selects unit tests and PlaybackLabTests only. Full 73-test suite passes
9e79f9cc2e4243508ce00fc40bf3562f; unsigned device build passes
a7f4ca94d24f4a039407faa0935910ac.

Remaining: exercise Restart, failed-save/disappearance UI, ten encrypted relaunch
cycles, and account intent journal/conflict/retry/isolation. Native private replies
share one ticket slot: assess long subtitle discovery competing with playback
commands (busy is unadmitted; cancellation/timeouts are not rollback). Do not infer
safe replay from stale tokens. No physical suspension/seek/playback qualification.

CI-speed helper verified prior 7m43 run reused exact artifacts and skipped Cargo
setup/seeder build. Core-only change in 07e7d20 invalidated the combined artifact cache,
recompiling seeder (71s) and torrent (26s). Next potential optimization: split Core
and torrent/seeder artifact caches with complete inputs. First measure Swift-only
push reuse of existing bb38dc cache; no workflow change in this slice.

CI component-cache follow-up: isolated `codex/apple-ci-component-cache` splits final Core artifacts from torrent/seeder artifacts, preserving full checks and independent source verification. All19 portable tests and workflow/shell lint pass; remote cold/full-hit/partial-hit validation remains required. See APPLE-CI.md.

### Phase 5 — playback waits for unadmitted private tickets

The shared reply slot was a confirmed integration problem: while owned subtitle
HTTP was held, playback immediately failed busy. Extended the actual native HTTP
fixture with two pending playback requests: cancel one while the slot is occupied,
then release subtitle discovery and require the other to reach Core's expected
stale_playback error for the already-closed token. Red
96a4d5b81ad04c989ce41d1a8500bdad returned busy for both assertions.
NativeCoreService.playback now waits in cancellable 20ms intervals for up to five
seconds ONLY while its ticket is zero (nothing admitted). Each retry verifies the
same live native handle and no retirement. Once admitted, existing receipt handling
runs once; no replay follows cancellation, timeout or native errors. Other service
operations retain their existing immediate-busy behavior. The admission wait and
existing five-second reply wait are separate bounds.
Full 73-test suite passes 7a53703eb6cc4e2899281c89cf7347fa; unsigned device build passes
8dcf18302f29468e94c1b54ad193f25c. Ambiguous accepted operations still need reconciliation;
this change only resolves contention before admission. Prior playback UI/relaunch,
account journal and physical gates remain as recorded above.


### Phase 5 — Restart decoder verification

Extended the opt-in selected-episode journey after successful Resume: reopen the
source, choose Start from beginning with the remote, wait for actual decoder time
of at least one second and require it below five seconds (the saved position was
at least eight). Close then restores source focus and Back restores episode/title
focus. Full peer journey passes 36c273e643f84f639258832de78b9454. No production change
or routine-CI expansion in this slice. This establishes Restart in the running app,
not encrypted process relaunch or physical suspension. Next: a dedicated debug-only
persistent fixture namespace and opt-in relaunch measurements, then account journal
and ambiguous receipt reconciliation.


### Phase 5 — encrypted process relaunch fixture

Debug JourneyFixtureView accepts only a UUID in CUE_JOURNEY_STORAGE, using a dedicated
Application Support/CueJourneyTests/<UUID> directory and CoreStorage's encrypted
state/Keychain path. Normal guest/account directories are untouched. Without the
variable the existing in-memory fixture remains unchanged. A cleanup-only launch
with CUE_JOURNEY_CLEAR_STORAGE=1 removes that UUID's test storage and key before
creating any Core owner; the UI test waits for its acknowledgment and terminates.
Invalid UUIDs fail instead of falling back to another store.

New opt-in testProgressSurvivesProcessRelaunch plays the peer fixture for at least
eight seconds, closes durably, terminates the app, launches the same test namespace,
navigates back through catalogue/title/episode/source, and requires Resume. It then
runs cancellation/preserved-label, actual resume within one second of the saved
clock, Restart and Back-focus assertions. The test failed at missing Resume/Restart
against the original in-memory fixture (808eadbc37ac430ba433207eff9619dc). With
encrypted persistence it passes 70e56573aad544ea852d45120297b5e2, including explicit
cleanup. Unsigned tvOS build passes 537200b5b3cb43a6bfc21da0e397506e. Routine CI still
excludes these long journeys. This proves one simulator process-relaunch cycle,
not ten cycles, cache purge, Home/sleep or physical-device qualification.


### Phase 5 — ten simulator relaunch measurements completed

Opt-in command `python3 scripts/fixtures/check-resume-cycles.py
'platform=tvOS Simulator,id=2058C8D1-FCB2-4384-ACD2-B86EF556BE60'` passes ten independent
real-peer journeys with encrypted UUID-scoped state. Each uses graceful Close before
process termination/relaunch, then Resume/cancel/Restart and cleanup. All ten saved
and restored the displayed9s position (0s loss at whole-second display precision).
Decoder resume observations were8s or9s, within1s of restored state and the15s bound.
Canonical sanitized results: [resume measurements](../standalone-tvos/evidence/resume-cycles-2026-11-18.json).
Exact local task evidence18e4aa17200547fba0f811c571756cec; raw per-cycle logs under
.agent-work/resume-cycles/4d6831bc4f544eb4b9488e4a5c088beb. Before/after content digests
match; isolated journal development did not alter the tested app between cycles.

Scope: Xcode26.6/tvOS26.5 simulator, owned36s H.264/AAC peer fixture, media cache may
be warm, separate encrypted fixture namespace each cycle. This is not a cold-cache
qualification, abrupt-kill checkpoint measurement, Home/sleep test or physicalA2737
proof. Account journal/network conflict/retry and failed-save UI gates remain open.
Routine CI does not invoke this runner. PR13 head64c4e1f is green; PR14 cache warm-hit
validation remains underway. Next bring the isolated journal commits ee21c5d and
0cb8c21 into the phase5 branch and validate full native/device builds.



### Phase 5 — isolated pending-revision journal foundation

Developed in the detached phase5/.agent-work/phase5-journal worktree at64c4e1f while
root's ten-cycle simulator run holds its inputs fixed. sync_journal.rs plans immutable,
UID-scoped mutations: at most256 titles/1MiB encoded storage, coalesced per-title
revision, monotonic counter retained after acknowledgment/reopen, exact matching
acknowledgments only, and strict schema/identity validation. No credentials, stream
URLs or duplicate library payload fields exist in the journal schema. Full intended
state remains in Core's library; integration MUST persist it and the pending revision
in one encrypted batch. The component is not wired to storage or network yet.
Missing-API red c4c3cfcf823f433087f7c83c5737a68a; first stale-ack test green
ed7b0666132e45e4bfb1f4a0b7131ce6; all six journal tests green
0aca01cdecce4a4084493c76d1b742f2. Evidence lives in that isolated worktree's task
phase-5-sync-journal. No concurrent Xcode build was run. Before integrating into the
root branch: finish its relaunch qualification, preserve its measurements, then
merge this code and run full native/device checks. Next implement atomic journal+
library persistence and suppress/replace Core's unjournaled concurrent account
writes; add revision-matched network acknowledgments, remote-merge policy and backoff.


### Phase 5 — atomic library/journal storage seam

Added checkpoint::write_with_journal in the isolated journal worktree. It validates
account UID and every pending title/type against the paired Core library, then writes
recent bucket, older bucket/deletion and cue_sync_intents_v1 in one encrypted batch.
The existing checkpoint::write path is unchanged; session callers have NOT adopted
the journal yet. Guest, different-owner, missing-title and type-mismatch pairs are
rejected before admission. Journal structural bounds are revalidated at this seam.

Missing API red c61684accc5c420eaac337d034ecb77c also exposed fixture UserId conversion;
88f0f7cf6b7345ea9a81af47f0a628b8 corrected UserId/newtype and StorageBatch visibility.
Encrypted directory-failure/reopen test passes507a1218fb264e14bd3a9ebe51c5a307: both old
position100 and revision1 survive failure; retry reopens at200/revision2. Quota test
leaves enough room for the new library plus OLD journal, but not the entire new pair.
A deliberate two-write mutation failed8315557ee36c4ca5a193f637b71ec0dd with position200
instead of100, proving sensitivity to a lost-intent partial commit. Single-batch
implementation restored; all native library unit tests pass
b1d70673335a4e788f51a9d78c800be6. Full native integration/device checks await transfer
into root after its ten-cycle run. Network scheduler, journal restore and routing
all account writes through this seam remain required.


### Phase 5 — journal foundation integrated after relaunch qualification

Preserved the ten-cycle results in84446df, then cherry-picked isolated journal work
as02766e5 and1ac0bb6. The sole handoff append conflict was resolved preserving both
the measured relaunch section and journal notes. Full native suite passes
58de8cd7718f452fae856284af5237a3; unsigned tvOS build passes
d1d6265ed50b45c090aac9fd89be08bd on the integrated tree. Journal/atomic writer remain
unwired to account-session mutations and uploads; their passing tests do not establish
account sync. Next implement restore and admission routing, conflict merge and network
revision acknowledgments/backoff. No additional simulator qualification rerun is
claimed for the dormant journal module. Root unrelated dirty documents remain intact.

- 2026-11-20: Phase 5 now contains the validated component-cache branch (PR14 merged into Phase5, not main), ten graceful encrypted resume cycles and atomic journal foundation at remote9b55c73. Added startup validation of any persisted journal against the restored library; guest/foreign journal regression fails before fix (551e46f870e74ec6b55364779cd0982d) and preserves stored data after rejection. Full native suite35f14af14d6b4f2fa887095ebccf5605 and unsigned device buildc708b8990698441294b561350f706364 pass. This validates stored intent; session ownership, account mutation admission and network scheduler remain unfinished. PRs3–8/13 have no review threads; GitHub now marks7/8/13 non-draft, which does not establish release readiness.

- Account playback checkpoints now read the last committed journal on the sequential storage queue, coalesce a new revision for the selected title and atomically persist it with Core library state. Two operations prepared before either executes retain both revisions; foreign journals fail without changing disk progress; guest checkpoints create no account journal. Missing-intent red0c20d62fc43545b59a5cc5eca1bd9c9c; full native green8f849c97fc7e4f14a4ebe1f6daa81219; unsigned device green5fd92533ccba4ebf8c53ed6e8da1a0b3. Remaining: manual membership/watched writes still use upstream effects; remote merge/upload suppression, durable acknowledgement, capped retry and sign-out handling need integration. Capacity failure preserves committed storage but in-memory checkpoint admission still precedes async failure; move capacity admission to session ownership before release. Do not claim account mode complete.

- Account library UpdateLibraryItem effects now retain Core mutation semantics but replace lazy upstream local/API futures with one atomic journaled storage write. Manual unchanged membership/watched retries carry the selected title through the message drain, so retry persists intent without reapplying watched counters. Guest effects remain upstream. Regression confirms the previous concurrent upload escaped (redf36061be05fb45cea224e51c6a31f074), then one sequential write, pending revision and unchanged retry persist correctly. Full native suitee3ee110f405f40b3ab1096fcc96c8f42 and device buildfa9543408cd74a92bc7727b1e87624db pass. Remaining upstream sync/pull paths must still be replaced or filtered before account release; this does not yet supply an uploader. Capacity preadmission, pending-local-wins conflict policy, matching-revision ack, capped backoff/auth pause and pending journal sign-out handling remain required.

- Added journal reconciliation planner: pending local title state wins regardless of newer/equal remote timestamp; without pending intent the remote record wins even if its timestamp is older. Pulls never acknowledge a pending revision. Duplicate remote IDs, conflicting title types and foreign library ownership reject the whole plan without changing inputs. The pinned Core merge reproduced loss of pending progress (900 instead of200, redc887491c425747ea85be4f0e0e4f9f62); final journal suitec8750ed8ab3644848c69d628ba733dda passes. Planner is not yet connected to session LibraryPullResult: session ownership of admitted but uncommitted intent is required first to avoid races. Do not claim remote conflict handling is active.

- Session now restores and owns the admitted journal. Account manual mutations reserve identity/capacity/revision before Core changes canonical library state, and playback checkpoints reserve before replacing ctx.library. Writes carry the exact reserved journal snapshot; failed saves retain admitted intent in memory. Regression first observed no reservation before effects (red06b070aee32d4c71937aa7de10d79edb); test now verifies reservation before execution, unchanged retry, and full-capacity rejection without inserting a title/changing the journal. Full native suited642f496ed7546a0afdf5949ad777763 and unsigned device73b262f1a8f846e183b1a29e5c2d1f15 pass. Next wire LibraryPullResult through reconciliation using this session-owned journal, suppress timestamp-plan uploads, and add uploader/backoff/exact ack. Manual playback watched override still occurs before downstream capacity rejection and needs consistent admission handling; sign-out/pending journal handling remains open.

- Connected valid-account LibraryPullResult to session-owned journal reconciliation and atomic library/journal persistence. Pinned Core account-key guard and Get-command guard remain; failed planning leaves state untouched. Remote pulls preserve admitted local intent even before its disk future executes, never clear pending revisions, and emit local failure on persistence errors. Race reproduced overwritten watched flag0 vs1 (rede1c4d1126e834566b873be026b148d17); full native6275274b735a41ccac4a36da2ba6f0c0 and unsigned devicec6ab02010cb1473c9a756562e8b9f1c5 pass. Still required: suppress timestamp-plan uploads, uploader and durable revision ACK, backoff/auth pause, sign-out journal behavior and manual watched override capacity consistency. Existing CI44784869553 remains live on9b55c73; local commits intentionally await that run before next push.

- Suppressed timestamp-plan account uploads while preserving Core pull discovery. Regression on LibrarySyncPlanResult first reproduced an unjournaled upload future (red32402b289a0a40b495ab1bdd4a9bb2e8), then verifies push-only plan has no network future and combined pull/push plan has exactly the pull future. Full native470d856ac0ec421c99918321fa7b2ba2 and unsigned devicebf097bb73b86403cbf4504d24a801fd4 pass. Account local changes remain queued until the revision uploader is implemented; this is intentionally incomplete account sync, not a release-ready substitute. Next add durable-snapshot uploads, exact-revision acknowledgements and capped retry/auth handling.

- Added immutable single-title upload snapshots (no Debug/history logging) with exact owner/revision acknowledgement, oldest pending revision selection, and a sequential-queue committed-storage reader. Tests prove unsaved state is not captured, foreign account storage rejects, saved snapshots remain immutable, and old acknowledgements retain newer revisions. Missing-API red049ad388430c4813938b6214279792a0; full nativea809647e6dff48afadfa53a2d54f7c26; unsigned device1c6a388ddb194e389e888996ef4761b6. No network upload/scheduler is wired yet; caller must run committed_upload on the sequential queue before dispatching a concurrent request. CI helper apple_ci_speed errored for workspace credits; root must inspect current slow run44784869553 directly when terminal.

- Added durable exact-revision acknowledgement write: returns the cleared journal only after atomic library/journal persistence succeeds, without mutating caller state on failure. Regression makes encrypted storage unavailable, verifies pending intent remains, retries and reopens to prove durable acknowledgement with progress retained. Missing-API red0d2f2510dd5445ee830877f8ff8b307f; full native638bda4da76b479f87597f8e427df122; unsigned device79ad477fcad44743a37d174dfbe68156. Network scheduler is still absent. Integration must gate stale completions by session identity, preserve newer in-memory revisions when ACK completes, and prevent earlier ACK snapshots overwriting later admitted writes.

- Added bounded one-title Stremio datastore upload transport using the immutable snapshot. Rejects wrong-owner/empty credentials before HTTP; sends exactly captured progress over Core HTTPS API; requires true success response; returns sanitized Identity/Rejected/Transport errors. Server/malformed responses retain journal; no implicit ACK. Regressionredbaec486a1a2a44eaa59cbdcaf90e5b7f; full natived0db772a7ecf4b119a4ec6bea35f735d; unsigned device9d248edb55f743f5bc8ca015355686fd. Transport is not yet scheduled by CoreSession. HTTP status authorization classification remains generic Transport because NativeEnv currently hides all non-success status codes.
- CI44784869553 passed on9b55c73: simulator18m48s, device/native4m31s, Android4m12s. Both exact artifact caches missed in this PR scope; seeder step173s, setup85s, simulator selection81s, build/test713s (testing308.8s; UI12tests246.8s). Complete local log .agent-work/ci-44784869553-simulator.log. Next push should exercise torrent partial-hit, Core miss; do not claim speedup until observed.

- Added deterministic sync scheduling policy: one in-flight ticket, monotonic capped1/2/4/8/16/30s retry, success reset, stale-ticket rejection and server-rejection pause. Explicit retry cannot bypass rejection; owner retirement/reauth invalidates callbacks without resetting the ticket counter. Missing-API red30e4b8da617f4e23b79778602267dbb6; focused scheduler8514a87de51d499a8fdffe541f771bdd; unsigned device4892ba80c0944597b4c03c9ee9f27e46;19portable tests pass. Not yet connected to CoreSession timers/transport. Integration must cancel retired-owner work, retain the gate through durable ACK and expose recoverable sync state. New CI44788112049 on e2c17d8 confirms torrent artifact verification/install success and seeder compilation skipped, Core artifact miss; elapsed speedup still unmeasured until terminal.

- Native HTTP now distinguishes401/403 with a fixed redacted authentication-rejection marker; upload transport maps it to Rejected, preserving retryable429/503 as Transport. Real loopback HTTP regression reproduced missing distinction (red1e463e96100842508c7064d43480aeb0); transport mock verifies classification preserves pending intent. Full nativea9e1880cbf894cf1be0f49597cdbb898 and unsigned device67c7623c26044f52bb01273ca8171030 pass. Scheduler still needs session integration; this closes the prior generic-status classification gap but does not yet pause a live session automatically.

- Connected CoreSession to committed snapshot preparation, concurrent single-flight upload, capped scheduler and durable ACK. Restored pending intent starts without UI action. ACK waits for pending Core effects to deliver, then uses a separate oneshot on the sequential writer while the actor admits no mutations; the captured current journal protects newer edits. Request gate remains live through ACK, read/network/save failures retain intent, server rejection pauses. Logout disables new sync and cancels/invalidates current callbacks; owner exit cancels work. Integration red593ed121402c4ed5a7072d3d73c04b8f showed restored journal never acknowledged; now exact one-request/durable-clear test and rejection/no-retry test pass. Full nativeec9dd25c7efe49db9b7dbf7d0f115bcb; unsigned deviceb2de9d5e1133455cb0be8b40d3807cd0. Remaining: adversarial late-upload/newer-edit and ACK-barrier queue tests, foreground/reconnection/explicit retry UI, persistent sync status distinct from transient core errors, pending-journal sign-out cleanup, manual watched override capacity consistency and lifecycle faults. Do not claim Phase5 complete.

- Added test-only async HTTP boundary and actor library-message seam for deterministic races (both cfg(test), absent from app builds). Integration test holds upload100, admits/saves200, releases old reply, proves revision2 remains pending until the second upload completes, and observes exactly[100,200]. Deliberately acknowledging current instead of captured revision fails waiting for the required second request (red0196726cfc444c90bf0b5aa299170153); restored implementation full native98941c5d66434997acd66f13b3dfae87 passes.19portable checks pass; production code is unchanged from the prior successful device build. CI44788112049 passed on e2c17d8: simulator8m50s vs previous18m48s; confirmed torrent cache hit/seeder skip (do not attribute the entire timing difference solely to cache).

- Added suspended-upload logout regression: cancellation drops the HTTP future, retires identity and preserves unacknowledged journal until explicit account cleanup. Removing logout cancellation makes it fail (red1a2ddf5c85244a5f99bb3ecd7ef23118); restored sync integration suite0cec9a1270b34f3983bbb56312dc8544 passes. Reviewed Swift signOut -> owner stop -> encrypted namespace/key deletion; tests preserve another account and retry cleanup failures. Initial local73-test run32791f8d6732443fa22d44750809fca3 failed only magnet metadata/bytes because the documented fixture peer was absent (port42424 had no listener). Restarted owned seeder (exec58427, PID30860, ready127.0.0.1:42424); unchanged suite c8fc280ed1bc4074a5faff53eb2f68f4 passes all73. Revalidate seeder process before future runs. This is component-level cancellation/cleanup evidence; physical sign-out/lifecycle gates remain open.

- Native sync status is now independent of transient Core errors: snapshots expose pending title count and idle/syncing/waiting/reauthentication_required, with no status for guests or stopped owners (including panic cleanup). Version-1 retry_sync resets eligible backoff but cannot bypass authentication rejection or overlap a flight. Regression verifies rejection remains visible after a language-setting action and explicit retry is refused; corrected the test fixture to ISO-639-2 fra (fr is intentionally rejected). Full native suite aca6155578a745ffb634e497314a69de and unsigned tvOS build df228f19a587417495ded11f6d282b98 pass. Swift decoding/banner/button and successful transient retry integration coverage remain to be added; no claim of complete recovery UX or reauthentication. PR7 latest checks verified green. PR13 run44789831206 simulator job136474241974 remained live at inspection; device/Android/harness passed.

- Swift now decodes native sync state, displays pending/syncing/authentication-paused banners, and dispatches Retry sync only for a waiting authenticated owner. Session generation guards suppress late admission failures; restart/replacement reset the in-flight UI flag. Guest retry integration verifies asynchronous FFI rejection in the snapshot (dispatch itself acknowledges queue admission only). Initial missing-API red8e33af63fabe419799cbadc33371d72a; corrected compile error b28a3637bda44881a819997dd375ecb4 and incorrect synchronous-error expectation d590e9f3f77f47598dc960d0cae97c24. All74 Swift tests ec96eba5bf904246be7e51c74adff86f and unsigned device build eff9f00f35d94c72945a420799fc30da pass. Owned seed PID30860 was verified listening42424. No successful account retry UI journey or same-UID reauthentication is claimed: those, foreground/reconnection, failed local-save replay, ACK-barrier races and lifecycle gates remain. Remote56d08eb passed all CI, simulator11m58s (torrent cache hit/seeder compile skipped).

- Fixed manual watched admission at full sync capacity: validate journal capacity/revision/identity before changing the active playback override or Core details. Synchronous effect draining prevents an intervening actor command before library_update records intent. New real-session regression fills256 other pending titles, opens target metadata/source through mocked HTTP, starts Core playback, then rejects watched=true and proves subsequent pause still reports watched=false. Before fix it failed specifically because rejected intent changed the playback owner (a0421b4edac84062830c11ab79886ce8); after fix targeted3faec98236f543dfae7b19c6a58092b6, full native44cc1fdf07c142dfa605bfd2de8bca53 and unsigned devicef1e6f55e0f2f40efb8d1e4d852c76fd4 pass. Earlier fixture setup failures retained; no real API contacted. Existing manual watched/idempotence/storage retry tests remain green. All stack PR review threads were resolved/absent at inspection. Current remote45d94a9 CI44792285477 remained running; avoid treating earlier-head green as this head evidence.

- Explicit sync retry now repairs admitted but unsaved library/journal state before restarting uploads. It refuses while Core effects/upload are in flight, uses a separate receipt on the sequential writer while actor admissions are blocked, and leaves authentication rejection paused. Real encrypted-store regression moves the owned directory away, admits offset200, proves no upload beyond100, verifies retry still fails while storage is unavailable, restores the directory, retries and observes exactly[100,200], durable ACK and offset200 after reopening ciphertext. Pre-fix red0782042b952c4a0aa429adc57bf378c8 never uploaded200. Full native e9d5b70f587f4fd1b933bbc32b5fbc4b and unsigned device527771a7c6214062a53232dd7064834f pass; enhanced reopen assertion targeted7d612822ef4b45839618601699007f43 passes with unchanged production code. Automatic foreground/reconnection retry, same-UID reauthentication, busy retry UX, and adversarial write-barrier/sign-out coverage remain. PR13 remote45d94a9 device/Android/harness checks passed; simulator job136482789659 confirmed actively executing tests. Capacity fix and this recovery fix remain local pending the next push batch.

- Added deterministic retry write-barrier regression: block the real sequential queue, enqueue Retry then a newer edit, verify neither receipt completes while blocked, release and confirm durable newest offset200 plus empty journal. Against the prior Retry implementation the test fails because the newer mutation crossed the barrier (220bb0caf86c44f79b132560aa5eb8be). Restored production implementation before further testing; targeteddd85fc8d5ac24938b2500c172a515626 and full nativea24ff9365b3a4d10930bd15913f82d64 pass. Only test code changed; prior device build527771a7c6214062a53232dd7064834f remains applicable. Backup .agent-work/session-with-retry-barrier.rs is not an active mutation. ACK-specific barrier and stop-during-barrier tests remain distinct gaps. Next substantial recovery work is same-UID reauthentication preserving admitted/persisted edits; existing new-account flow creates a new namespace and must not be reused by discarding old pending state.

- Reauthentication foundation (not yet wired into CoreSession/FFI/Swift): reauth::authenticate calls pinned Core login API without new-account state replacement or request/response logging, bounds credentials, sanitizes errors and only returns a same-UID candidate with a bounded nonempty key. checkpoint::refresh_account prepares one atomic encrypted profile/library/journal replacement, independently rejects foreign identities and retains all pending revisions. Test verifies failed encrypted write leaves old credentials/library intact; retry and ciphertext reopen retain replacement credentials, offset200 and pending revision1. Removing transport UID guard fails2aadda38a0d84c4fa316fd2b4968bc5a; guard restored before full native65228ec614a1416d9079def54399215d and unsigned devicefaef5ec9a32546ec91ec62ba2cd11e5c pass. Still requires cancellable session operation, stale-result ownership checks, durable publication barrier, scheduler unpause only after success, dedicated credential FFI and UI. Existing new-account flow must not discard pending state for reauthentication. Recovery commits through71e846c pushed to PR13 and remote blobs verified; preceding45d94a9 allgreen (simulator10m6s). New CI44794075459 active; no current-head green claim.

- CoreSession now admits bounded same-account reauthentication only for an authenticated paused owner. Concurrent login uses a cancellation token and monotonic ticket; callback checks active ticket/UID. Candidate credentials wait for all Core effects, then commit current profile/library/journal through a separate sequential receipt while actor mutations are barred. Only successful durability replaces live profile and unpauses sync. Failure retains old credentials and pause; logout clears/cancels candidate and run shutdown cancels request. Integration covers successful refresh/new-key upload, foreign UID rejection with pending intent intact, and encrypted-store failure preventing unpause. Missing API redfa98ea2522d0479cbdce2363335f995e; full native3c9a16ade9c54f38aed612f443c644ef passes. Device build3953ed867a3f4e25908822a331a8c993 failed solely ENOSPC; removed only task-owned host Rust incremental cache (core-target/debug/incremental), unchanged build f7d9c065357141d19289b84052d01bcc passes. No source/evidence deleted. Still NOT exposed through FFI/Swift: add progress/completion observability, held-login logout/stop and queued-write barrier tests, credential FFI, and same-account form. Existing snapshot sync remains reauthentication_required during native refresh; UI must not infer completion from admission.

- Added native sync.reauthenticating progress, preserved across ordinary snapshots/settings actions and cleared by owner retirement. Swift decodes it optionally for compatibility and shows Signing in in the paused banner. Held-login integration proves logout and stop drop the auth HTTP future, clear identity/progress and preserve unacknowledged journal until explicit cleanup; concurrent refresh is rejected. Removing logout reauth cancellation fails timeout041f17ab42c741c19ef7ab7d8451e2c7; restored before full native9f767e5332354c70b4e04858f49b747d,74 Swift tests df82625410b449e597d6124c79de3897 and unsigned device6ffc0bbce5d441f28b8262baa92331d0 pass. Missing progress API red9c6885aa550749fa9dfb4ec2a0ca31c0 retained. Mutation backup .agent-work/session-before-reauth-cancel-mutation.rs is inactive. Disk pressure resolved by deleting only rebuildable tvOS incremental caches plus obsolete Nov17 core-target/aarch64-apple-tvos{,-sim} outputs after verifying current scripts/linker use native-target;4.9GiB free afterward, source/test evidence preserved. Seed PID30860 verified before Swift suite. FFI credential entrypoint and form remain unimplemented; no UI sign-in success claim. Current remote71e846c simulator CI44794075459 remains running; device/Android/harness passed.

- Added cue_core_reauthenticate credential FFI and Swift CoreService.reauthenticate. Shared bounded credential copying preserves original login contract, rejects control characters for refresh, keeps refresh out of JSON dispatch and exposes only queue admission. Native regression checks malformed/oversize/control/UTF-8/null inputs, private JSON rejection, guest asynchronous refusal, redacted snapshots and retired handles. Missing-export redb1e86a0001a24c84a8c3688b19a0be7d; focusedcd3f6048628843359ae74b51604fbbdc; full native35402d2a27c6474c895fe55344adbf9f pass. Existing Swift new-account test verifies refresh input rejection and native refusal before account authentication; all74 Swift4431819ef8b748ee987c0f3d9ffe49f2 and unsigned device1f2bdec32485498191e9cee465dde979 pass. Seed PID30860 verified. Next add same-owner refresh form, stable operation completion/error handling and explicit cancel without sign-out; no form/live account qualification yet. Remote71e846c allCIgreen (44794075459 simulator10m31s/device4m46s).

### 2026-11-20 — account refresh form verification (work in progress)

Phase 5 worktree, base `2231f12`: the uncommitted same-account refresh form and numbered native cancellation now compile and pass simulator tests. Fixed catch-variable shadowing in `AccountReauthenticationModel.cancel`; the original compiler failure is retained as evidence `5e130f2b27ef4e37a2b2ab69d7d23acd`. Rerun `python3 scripts/agent/task.py verify phase-5-library make apple-test DESTINATION=platform=tvOS Simulator,id=2058C8D1-FCB2-4384-ACD2-B86EF556BE60` passed (`c7ce47ecdd964078a9dadc4719977428`). `make check` passed all 19 harness tests and `git diff --check` passed. The controlled fixture seeder was live on localhost:42424.

The form test verifies guest rejection and cancellation without retiring the guest session; it does not prove a successful connected-account UI journey. Earlier native targeted cancellation evidence is `49a3e46df5dd4094a46719ce8221c94d`. Next: review admission/cancellation failure retention, run full native and device build verification, then commit and update PR #13. Current form changes remain uncommitted. Physical gates and later phases remain incomplete.

### 2026-11-20 — refresh cancellation review and native verification

Full locked native suite passed (`3d18d337d0c7404b8e2f88b33a59b5d1`); unsigned tvOS device-target build passed (`54aec0e5de3b41cfb07aa1bf8f45e349`). These are local software checks, not physical-device playback.

Act before shipping the uncommitted form: `NativeCoreService.reauthenticate` catches admission cancellation/timeout and uses `try? dispatch(cancel_reauthentication)`; a full native command queue can reject that cleanup, but the original error escapes without retaining the expected sequence. `StandaloneModel.beginAccountRefresh` similarly suppresses cleanup failure after admission when its task/generation changes. The form then has no sequence to retry. Preserve cleanup ownership across these error paths and prove it under a held admission/queue rejection. Also, `cancelReauthentication` currently treats an older snapshot sequence as completion; that is not acknowledgment of a queued cancel during admission. Use an explicit native acknowledgment or equivalent ordered completion evidence, and keep obsolete cancellation scoped to its attempt. Existing success tests do not cover these failures. Changes deliberately remain local pending this repair.

### 2026-11-20 — explicit refresh cancellation acknowledgment

Implemented `cue_core_reauthentication_cancelled`: last sequence acknowledged only after the session consumes cancellation. Swift now waits for this receipt instead of interpreting a potentially old snapshot as completion. Admission and post-admission cleanup failures propagate `refreshCleanupRequired(sequence)`; the form retains that sequence for retry rather than silently dropping ownership.

Missing-API red: `654a1328d56b4b3691153d2823f10dfa`; focused FFI green: `aac7a94a9fe848fdb04999ac789e83e1`. Simulator suite passed `4358cce93d65453ca88dfc1d6093b21d`. Full native verification launched in exec session 32595; recover its terminal result before restarting. Still needed: fault-injected Swift cleanup retry test, device build after this ABI addition, final review and scoped commit/push. PR #13 remote head 2231f12 checks were all green; Apple simulator job 136497297608 took 6m7s. Local changes are not yet pushed.

### 2026-11-20 — refresh form shipping checkpoint

Full native suite completed successfully (`f465d73b74fb48b6a18c5ef7d79ba71e`), superseding the live-session note above. Added deterministic real-queue pressure coverage: rejected and merely queued cancellation produce no completion receipt (`a91537d52e0146eba4f36ae0195f993e`). After Rust formatting, unsigned device build passed (`4c9e644ffde540a99d354094c9f7d85c`). Simulator evidence remains `4358cce93d65453ca88dfc1d6093b21d` (75 tests).

This slice adds same-account refresh UI, attempt-scoped cancellation and native completion acknowledgment, retaining attempt identity on failed cleanup. It does not complete Phase 5. Next: fault-injected Swift cleanup/retry ownership coverage and successful refresh UI journey, then foreground/reconnection and remaining lifecycle/ACK gates. No physical qualification is claimed.

### 2026-11-20 — durable upload ACK writer barrier regression

Added a real-session regression that holds the sequential storage worker, waits until the upload ACK write is queued, then submits a newer edit. The edit cannot complete across the barrier; after release, the stored library contains the newer position and outgoing payloads are independently checked as [100, 200]. A test-only queue-depth observation exposes existing writer state without replacing persistence or session logic.

Focused green `6c63d6ee92894c6ea63da06e356874b2`; deliberately bypassing the ACK receive wait failed with “New edit crossed durable ACK barrier: Ok(())” (`08ec481cf4ae4ffe94e51cd434136c94`). Production code was restored; full locked native suite passed `6ec83ba8cf1c45158cc8f7f76eb2ba4d`. This is in-memory storage ordering evidence, not physical interruption or encrypted reopen qualification. Only test code and cfg(test) instrumentation changed, so no additional Apple build was needed for this slice.

Review audit: PRs 3,4,5,6,7,8,13 have zero unresolved review threads, with no further pages. PR13 head d09af57 CI was still running. Keep remaining cleanup fault/UI, lifecycle and physical gates open.

### 2026-11-20 — foreground sync retry wiring

StandaloneHome now observes scenePhase with a cancellable SwiftUI task and calls the existing guarded retry when active. StandaloneModel admits this only for an authenticated waiting account, with no sign-out or retry in progress; native guards independently preserve rejection pauses and serialize the durable retry save. Initial activation before state restoration is harmless because native startup schedules restored pending work. Network recovery without a scene transition continues to use native capped retry/backoff; no path monitor was added.

Simulator suite passed `a8e2504c40ba4da2a7633a20e8f7f8fe`; unsigned tvOS device build passed `43bfceb273d64c009bc63c520b148949`. This wiring does not prove a connected-account foreground journey or physical Home/sleep behavior. PR13 d09af57 Apple CI jobs were confirmed in progress (44799070585), building device libraries and running simulator suites. Remaining qualification gates stay open.

### 2026-11-20 — stop during queued durable acknowledgment

Extended the ACK barrier fixture to encrypted persistence and shared setup for two tests. Stopping with an ACK queued behind a held writer completes within the test bound, drops the held future, and preserves pending revision 1 and position 100 after encrypted reopen. The normal branch still verifies a queued newer edit and outgoing positions [100,200]. Initial fixture incorrectly read through the canceled environment (`b5a34948fcb547b2818fefff0c1cb6bf`); corrected to reopen storage rather than weakening cancellation. Focused stop test passed `e036ba5eafce472a954aa04c725d3e8d`; full native suite passed `4a4116a8d7e54ef0b2e3a409c7aa5dd2`.

Batch with local ACK regression and foreground retry commits for PR13. Foreground Swift/device evidence remains a8e2504c40ba4da2a7633a20e8f7f8fe / 43bfceb273d64c009bc63c520b148949; subsequent changes are native tests only. This covers cooperative shutdown while a storage write is queued, not abrupt OS termination mid-filesystem-write or physical Home/sleep.

### 2026-11-20 — diagnostics privacy audit and Android CI follow-up

Source audit: Core Env::log discards upstream messages; torrent FailureCode serializes fixed categories/messages; standalone playback displays fixed coordinator failures. Added a focused native test proving arbitrary sensitive error-chain strings are discarded while a typed low-storage category survives context (`16389ba540014e4f8513a2952505b2b1`). Test changes remain local.

Open diagnostic gates: Rust panic hooks may print payloads before catch_unwind (not yet bounded by app policy); VLCKit internal logging is unverified; native playback snapshot currently exposes verified/total bytes but lacks peer/rate/cache counters needed by the PRD diagnostic panel. These findings mean privacy/diagnostics are not complete. Legacy WatchModel localized errors are not used by StandaloneHome; do not confuse legacy code with the shipping route.

PR13 head484c067 Android run44800036472 failed before compilation resolving multiple Maven artifacts (including Kotlin/ASM). No Android files changed in this batch. Saved failed log under ignored .agent-work/android-44800036472-failed.log and requested a single failed-job rerun to distinguish transient artifact resolution from a persistent build issue. Apple run44800036321 was in progress. Recheck actual job state before further action.

### 2026-11-20 — VLCKit logging policy implementation

Inspected pinned VLCKit revision3d0f0fa7031eab1a67c817c98a495e1c68fc998c headers and VLCLibrary.m. `setLoggers` installs libvlc_log_set only for a nonempty logger list; empty removes the callback. Added a stateless, nonisolated discard-only VLCLogging sink and routed the shared playback coordinator through a factory that installs it on the shared library before player/media construction. All app VLCMediaPlayer construction now uses this factory. Core and torrent UI error categories remain available.

Simulator unit suite passed `75ceaafe087e4dc4a44ca67cc7318be8`. Changes remain local: run short playback tests and device build, inspect captured diagnostics with synthetic private inputs, then commit. This does not yet prove all third-party console output is suppressed; Rust panic payload policy remains open. Android rerun44800036472 job136510502916 was confirmed in progress in Build the TV apps after the prior artifact-resolution failure.

### 2026-11-20 — logging playback verification in progress

Started `task.py verify phase-5-library make apple-lab-test DESTINATION=platform=tvOS Simulator,id=2058C8D1-FCB2-4384-ACD2-B86EF556BE60` in exec session31002. Live xcodebuild is running; log `.agent-work/tasks/phase-5-library/e1d1e8b6ff72449da38313a088e91e0a.log` shows HEVC/alternate-track cases passing and continued tests. Resume this handle rather than restarting; no concurrent Xcode build. The local fixture seeder PID30860 remains listening on42424. After terminal result, run unsigned device build and record limits before committing privacy changes. GitHub Apple jobs136509881384/136509881743 and Android rerun136510502916 were confirmed active.

### 2026-11-20 — VLC logging verification complete for this slice

All12 short playback UI tests passed in115.941s (`e1d1e8b6ff72449da38313a088e91e0a`); unsigned device build passed `24575f698c2e4d0db0b7a2933bb4acaf`. The playback verification tree fingerprints differ because this handoff was appended while tests ran; playback source did not change during the run. Earlier75 unit tests and sensitive error-chain test also passed (75ceaafe087e4dc4a44ca67cc7318be8 / 16389ba540014e4f8513a2952505b2b1).

Commit the explicit discard-only VLCKit callback and error snapshot regression. These establish compatibility and the selected log callback policy, not a complete runtime console capture guarantee. Rust panic output, direct third-party stderr outside the callback, synthetic-sensitive-input capture and required diagnostic counters remain open. Android rerun136510502916 succeeded3m53s without code changes; current remote native/device CI passed5m44s, simulator136509881743 stillrunning.

### 2026-11-20 — allowlisted torrent diagnostic counters

Native ready snapshots now include a separate diagnostics object: engine_state from the pinned enum, live_peers and download_bytes_per_second from live stats (null when unavailable), plus verified_bytes/total_bytes. No upstream error strings, source URL, filenames or peer addresses enter this object; the private playback snapshot still carries its required playback fields. Rate uses rqbit Speed::as_bytes, whose source documents MiB/s conversion; no unit guessing. Verified bytes are not labeled cache allocation or playable buffer.

Peer-stream fixture failed before implementation on missing safe projection (`bcdd6f426d474254aa9a6c2cc7ecdb53`). All7 peer-stream tests passed after implementation (`7341ddd2609543f39c21a4cd5fb70a28`), checking exact field allowlist and observed counters after byte transfer. Changes remain local/uncommitted. Next: typed Swift decode, generation-owned periodic monitoring and compact diagnostics UI; add measured cache/buffer semantics without substituting verified-byte totals, then full native and Apple checks. Panic-output policy remains open.

### 2026-11-20 — typed playback diagnostics and live monitoring

Swift now decodes allowlisted diagnostic fields, using a fixed engine-state enum and optional peer/rate values. SourcePlaybackModel publishes initial diagnostics and owns a one-second polling task through SourceOwner. Teardown cancels and joins it; updates check both generation and closed ownership after await. New-source/stop clears old diagnostics. Native failure after initial ready now reaches the existing failure UI. The screen displays only engine state/counts/rate/verified totals; unknown values remain Unknown.

Simulator suite passed `6b9dadf060a34878b75a2b5f86927b2b`; controlled native-to-Swift fixture additionally asserts live state, known175917-byte total and available peer/rate fields. Remaining before shipping this slice: monitor stale/cancel behavior regression, device build, native checks and playback UI verification. Cache allocation and buffer measurements are still missing and are not replaced by verified totals. PR13 remote484c067 is fully green: simulator8m33s, native/device5m44s, Android rerun3m53s, both harness checks6s. Local logging commit6a1d657 and diagnostics work remain unpushed.

### 2026-11-20 — real-peer diagnostic replacement regression

Added a real-peer SourcePlaybackModel test: transfer known175917 bytes, wait for periodic diagnostics to report verified completion, replace with HTTPS, check diagnostics clear and remain absent beyond the old polling interval, then stop and reacquire cache. Shared only the controlled tracker fixture with the existing bridge test. Simulator suite passed76 tests (`c7bbef947f5e40b1978635546522c759`). Removing the new-source diagnostic reset caused the new test to fail (`38c28c260a7f4747a9df16ba9993a182`); production reset restored by finally. Next validation must rebuild the restored source (latest simulator executable contains the deliberate mutation).

Pinned VLCKit exposes mediaPlayerBufferingChanged(progress) in [0,1], explicitly a readiness percentage rather than buffered duration. This is the prospective buffer metric; do not invent buffered seconds. Cache allocation can be sampled separately from storage-budget disk usage without treating verified bytes as allocated size. Both measurements remain to implement.

### 2026-11-20 — measured cache and decoder buffering diagnostics

BudgetFactory now samples real filesystem allocation (data+directory blocks) at most once/second, returning unknown on failure/symlink; it does not call the injected quota probe or substitute logical/verified byte counts. Tests prove sparse-file block measurement and no symlink traversal (missing-API red9a61a4b53b11440b96f84ad7740568e7; greenf4b110fbc70840ceb410b1d6b3781072). Cache allocation is included in the native allowlist and Swift display.

VLC coordinator now owns the buffering delegate and reports a bounded finite readiness fraction via the existing generation-checked decoder snapshot. Stop disables delegate callbacks and clears readiness; source reset clears display. The UI labels it decoder buffer readiness, not duration/bytes. Callback test covers valid0.4, invalidNaN and a queued event after stop (missing-API red03f1a07514aa425d876c8fb98de17231).

Full torrent suite passed a393f7ce09814480b2729e68e743d2ad; simulator suite passed39cc583587fd4e15966201b813e4e12f after restoring the previous mutation. Next: unsigned device build, actual standalone playback diagnostic UI check (including readability/focus), final review and combined push with logging commit6a1d657. Panic-output privacy and remaining Phase5/physical gates remain open.

### 2026-11-20 — diagnostic UI qualification and shipping checkpoint

Unsigned device build passed e9b25428b99a4009b23725cebef0cbb0. Direct xcodebuild journey invocation failed before playback because required fixture servers were not started (35fc09908f6644ef80438b82d90cf4c4); correct fixture runner then passed unchanged app code: `bash scripts/fixtures/check-journey.sh platform=tvOS\ Simulator,id=2058C8D1-FCB2-4384-ACD2-B86EF556BE60 StandaloneJourneyTests/testSelectedEpisodePlaysFromPeerAndCloses` (5e643ddaeb374ea68709dde0e0375859). Use quoted destination as usual.

Journey assertions verify selected green episode pixels, playback/resume/Back focus, diagnostic readiness100%, available numeric fields, and no source identifiers in diagnostic presentation. Visually inspected exported playback screenshot: readable three-line diagnostics, no overlap with controls/subtitles/Close. Snapshot shows verified528KiB, allocated536KiB, decoder100%; these correctly remain distinct measures. Local screenshot .agent-work/diagnostics-ui-evidence/1754A39C-2E84-490A-9B95-AA8D7BD06FFC.png. Fixture script cleaned its server/seeder.

Ready to commit this diagnostic slice and push with logging6a1d657. Not full Phase5 completion: Rust panic-output policy, broader raw-console privacy capture, connected-account cleanup/reauth fault/UI journeys, abrupt interruption/cache purge and physical gates remain open. Current77 unit/native suite evidence is recorded above.

### 2026-11-20 — explicit private panic diagnostics and CI fixture repair

App startup explicitly calls cue_core_configure_private_diagnostics before native sessions. This opt-in replaces the process-wide Rust panic hook with a fixed “Cue TV: native panic.” line; it never formats payload/location or forwards the previous hook, and does not change unwinding/recovery. Installation is idempotent. Generic library users do not opt in automatically. C header documents this process-wide contract. The app checks installation success before creating views.

Child-process regression requests full backtraces and triggers caught synchronous/worker-thread panics containing synthetic private values; stderr is exactly two fixed lines and stdout lacks the canaries. Missing-API red8f9565b068d64263932deff838cf66d3; focusedgreen39fbd2ae715a4c6fa6fd309748210f65; full Core suite5d2c878e68884d04b59fb255bfa42288;77 simulator testsb9a15dd15b4343fa8abd2df76d4de9ab; devicebuild7bc851debe4c44dbba3e4c0502ad316e allpass. This addresses Rust panic-hook output; independent OS crash reports/direct third-party stderr are not proven private by this test.

CI native job136520339423 failed on the cache test assumption expected<8MiB, while exact allocated-block equality passed. Hosted filesystem eagerly allocated the extended file. Revised fixture adds a one-byte file, keeps exact st_blocks-derived equality and explicitly rejects logical length (8MiB+1 cannot equal a sector-aligned block total), supporting both sparse and eager filesystems. Focused cache tests passde4c89d60e814f8caf2917be7ec606a8. Failure log retained .agent-work/native-ci-136520339423.log. Ship this repair with the panic policy and recheck current-head CI. Other Phase5/physical qualification gates remain open.

### 2026-11-20 — current-state guidance and stack review audit

Corrected AGENTS.md's obsolete claim that Core/production playback remain planned. Replaced PHASE-5.md's stale opening backlog with the implemented checkpoint, exact local evidence references and outstanding software/physical gates; chronological slice records remain intact and explicitly historical. No product behavior changed. `make check` passed all 19 harness tests. GitHub review-thread audit found zero threads and no pagination on PRs 3, 4, 5, 6, 7, 8 and 13. PR7 current head2006470 has six successful checks. Phase5 head71af98f Apple run44803912934 and Android run44803912851 were still in progress during this audit; harness checks passed. Next: finish monitoring those specific runs and continue account-cleanup ownership qualification. Native test HTTP injection is currently cfg(test), so a simulator account fixture must use a deliberate network-boundary seam rather than pretending a mocked Core service proves the real bridge.

### 2026-11-20 — isolated account HTTP fixture across the real Swift bridge

Added opt-in `account-fixture` Cargo feature with per-session loopback port, exact https://api.strem.io origin mapping and rejection of other destinations. Request paths/body, reqwest response bounds/status handling and cancellation remain real. Fixture client disables proxies and redirects. Ordinary sessions retain production routing. Dedicated FFI constructor creates an empty encrypted synthetic-account namespace; Swift exposes it only in Debug simulator startup. Simulator native build enables the feature; physical tvOS build forbids it at compile time. CI host Core tests enable fixture coverage; no long tests added. Native artifact key already includes both build script and source, so old cached binaries cannot satisfy this change.

Missing-API red3aaafabc0a644d579be8102ec894f392; real HTTP rejected-login green958f0f78eab84b2caecdb13eaf12973e; full Core+fixture suite49d21d8e89664ece8da2e9eb6cf986b6. Swift compile initially rejected nonisolated on actor(c9815b6c6ee84332bc7111b64083a794), corrected; all78 simulator tests pass9492c30dadb44fadb2ba4db318a839d2, including actual Swift→FFI→loopback successful login, encrypted file and ordinary identity-checked reopen. Fixture parser handles split request bodies and decodes password while tolerating the upstream facebook Boolean. Device build passesf51570075d2648ea9e86b1084027b7ef. Physical target deliberately built with fixture feature fails specifically on compile guard9bb281a74b12469b94d5e6d69f4f027d. Symbol inspection of extracted cue_core object members confirms fixture constructor absent from device archive and present in simulator archive. Whole-archive Apple nm cannot parse some Rust23 LLVM objects; extracted Core native objects were checked successfully instead.

Reviewed shell compatibility: macOS Bash3 errors on empty-array expansion under nounset, so build arguments are always a nonempty array; device build validates that path. make check passes19 and diff whitespace check passes. Removed only rebuildable host incremental caches to recover disk; retained task logs/fixtures/evidence. No external dependencies changed. Next: use fixture to drive paused-account reauthentication and failed-cleanup ownership through AccountReauthenticationModel; successful initial login/restoration does not establish recovery or hardware gates.

### 2026-11-20 — reauthentication cancellation receipt race fixed

Extended synthetic-account fixture through owned catalogue→details→library add→HTTP401 sync pause→AccountReauthenticationModel. Four cases now cover success, wrong UID with retry, encrypted-save failure with retry (temporarily move the test-owned directory), and Cancel while login response is held followed by immediate retry. Assertions retain account/session/title ownership and pending intent, inspect old/new API upload tokens and title IDs, require one success callback, and observe the held HTTP connection close with sub-second simulator Cancel. This is real Swift form-model→FFI→Core→HTTP/storage coverage, not a SwiftUI remote/focus or live-service journey.

Cancellation case reproduced a production race: FFI stored its explicit cancellation receipt before publishing the settled snapshot, so immediate retry read reauthenticating=true and failed invalidRequest. Red eee38a90e6c44d0fb83730cb42aed308; fix publishes settled snapshot before release-storing receipt, documented in C header. All four cases and all79 simulator tests pass958b7cda3ce34ffdbe77963b814db49e. Full native Core+fixture suite1f2c3c0a56434d2f95ad6196bf11c476 and unsigned device build0bf5004b03d440db981ce23b04761c9e pass; make check19 and staged path checks pass.

Earlier test-writing compile failure e7c19028032c47bf966e3f46c43e4953 used wrong catalog property, corrected. First runtime suite5941724eefa046f9a7c9ab62a584b3e9 passed new success case but four playback tests failed low_storage on the laptop. Removed only two rebuildable host Core object caches (397465c511a97ae1 and30120c0bd03f25b8 under core-target/debug/build/cue-core), recovering roughly6GB; retained evidence and source. Subsequent suite2b018418a3954a069137a101c245da3d passed before adding Cancel regression; final suite above passes all tests.

Next: faulted cleanup acknowledgment/ownership and full account UI journey, pending-journal sign-out, ambiguous accepted-operation reconciliation, interruption/cache purge and physical qualification. These remain open; the successful cancellation test does not prove retry after cancellation itself fails. Previous remote90d0bad native/device and Android jobs passed; simulator job136529609608 was still running at last observation.

### 2026-11-20 — pending-account sign-out and media cache-purge qualification

Added two real Swift/native/HTTP/storage sign-out cases starting with one rejected pending library edit and a held refresh request. Normal sign-out stops HTTP, suppresses success callbacks, clears selection and deletes only the retiring namespace/key. Fault case moves that fixture namespace and replaces its path with a symlink to another fixture account: cleanup rejects the symlink, keeps ciphertext and Keychain key recoverable, blocks account replacement, and succeeds after repairing the path. Other account sentinel and key remain unchanged. Original recovery test bootstrap was extracted without removing assertions. Initial80-test suitec5b3f3258c93418ba02118e8d6a60e22 passed; explicit Keychain retention/deletion assertions passedad2a93c42f4c467d9629c8e6f6f6c9fd.

Added media-purge composite: download all175917 owned torrent bytes through SourcePlaybackModel/native peer adapter, stop, remove disposable cache root, and download identical bytes again with HTTP response caching disabled. Pending account session, library membership, paused sync/one pending title, encrypted state bytes and Keychain key remain unchanged; no additional account upload occurs. Shared the existing controlled tracker helper across test files. Final81-test suite passes42c8772dcc1e47d782f1515ed86d4bb2 (34.180s test runtime); cache case5.297s. make check19 and diff whitespace checks pass. Test/document changes only; native/device code retains prior verified b4fa1a8 behavior.

This qualifies simulator/model boundaries, including cleanup failure on sign-out; it does not prove failed reauthentication-cancellation acknowledgment retry, full remote-driven account UI, OS purge during active playback, abrupt suspension or physical-device behavior. Next: full account UI fixture and remaining ambiguous operation/cleanup gates, then phase6/7 deliverables. At latest observation remote b4fa1a8 native/device passed; simulator run44807156461/job136534070697 was still running combined unit/short playback step. Wait for that run before pushing this tests-only slice to avoid cancelling near-finished evidence.

### 2026-11-20 — remote-driven account recovery UI journey

Added opt-in AccountJourneyTests and check-account-journey.sh with a loopback-only synthetic API server. Simulator Debug-only AccountJourneyFixtureView creates an isolated real account, loads owned catalogue/details and produces a rejected journal upload; it does not substitute UI snapshots. Existing production SwiftUI controls then handle native text entry, rejected password/error, password clearing, successful retry, library navigation and sign-out. Added stable accessibility identifiers to refresh form fields/actions. User-supplied credentials and live services are never used. Dedicated opaque namespace/selection is cleared in a separate test cleanup launch. The runner owns only its server PID, waits for its own readiness log (not an unrelated listener), retains that log, and asserts exact HTTP login/upload counts after UI completion. Routine CI selectors unchanged; journey remains opt-in.

First UI journey e5d6890dcbee42adbfddea558fbef4da passed35.155s, including server counters. Final runner with owned readiness passed804f8062beeb40709d4025d659f8a701. Unsigned device build2a7bdb853f204e33a2ebbe639922fa8f and make check19 pass. Actual screenshot exported and inspected: .agent-work/account-ui-evidence/1839A8C3-D16D-429B-99D5-44E95D8358E8.png (3840x2160): account label, retained title and focus readable, no overlap. Result bundle .agent-work/DerivedData/Logs/Test/Test-SittingRoomTV-2026.11.20_21-13-12-+0100.xcresult. Fixture server cleaned up after both journeys.

Remote dd47bde passed all six checks: Apple run44808787915 simulator1365396549685m05s, native/device1365396552145m10s, aggregate passed; Android44808787984 and both harness checks passed. Both exact native artifact restores were verified successful; preceding b4fa1a8 simulator was9m55s without the same reuse. These are observed run durations, not a controlled performance comparison. Remaining: first-sign-in UI/live account qualification, failure of reauthentication cancellation acknowledgment itself, ambiguous accepted-operation reconciliation, abrupt/physical lifecycle, broader console privacy, earlier phase gates and phase6/7.

### 2026-11-20 — first-sign-in UI and teardown qualification

Added a simulator-only account transport choice through CoreAccountLogin and AccountSignInModel, with Stremio as the unchanged default. StandaloneHome accepts a sign-in model factory so the fixture exercises actual candidate namespace creation, rejection cleanup, selection and account adoption through production controls. Stable sign-in identifiers also migrate the existing short cancel-form test without removing assertions.

Opt-in account runner now executes two remote/keyboard journeys: guest first sign-in (wrong password then success/catalog/sign-out) and paused-account recovery. Exact HTTP counts are accepted logins3/rejected2, accepted upload1/rejected1. Each journey has a unique parent namespace; successful sign-out must leave no selection or child directories before cleanup. Framework addTeardownBlock replaces Swift defer because fatal XCTest UI queries bypassed defer in the red run. Focus failures now throw before selecting an unintended control.

Red92d764ae4b08460d873e2027b00bb070 navigated Up from the debug Playback Lab overlay into that overlay; changed fixture navigation to Down based on captured focus evidence. Both journeys pass0a22a1b23881460880561bc6209fecdc (69.326s total), including HTTP counters and empty-state teardown. Routine suite passesabe31d8c80c141d3902b1741dd67084e:81 unit/native tests and12 short UI tests. Unsigned device build passes7a55ec9fb52f40fb8d6b859e5e16a0d2. No native changes or dependency upgrades. First-sign-in screenshot .agent-work/account-first-ui-green/9C5460A8-E392-4424-A149-B044CEB2D93E.png visually inspected: account/catalog content readable, focus visible, no overlap. Result Test-SittingRoomTV-2026.11.20_21-23-18-+0100.xcresult retained. Long account journeys remain opt-in.

Previous remote e96a5f7 all six checks passed; Apple44810059562 simulator9m12/native5m26, Android44810059516 passed. These results precede this new slice. Remaining gates include failed reauth cleanup acknowledgment ownership/retry, ambiguous operation reconciliation, live service, abrupt/physical lifecycle, broader console privacy, earlier-stage unresolved gates and stages6/7. First-sign-in simulator UI is now qualified; live/hardware sign-in is not.

### 2026-11-20 — dismiss refresh after verified session retirement

A new real-account/held-HTTP regression reproduced a cleanup dead end: cancel the running home/session while a refresh response is held, await its actual shutdown, then attempt Cancel twice. Both attempts failed because cancelReauthentication dispatched against an already released handle. Red67666abbae6b4369a0a468465feeb3c3 has exactly those two failures; source was edited after the failing assertions but before xcodebuild finished reporting, so its before/after hashes differ. The executable was built before that edit.

NativeCoreService now treats an absent handle as already cancelled, after validating nonzero sequence. In-flight or failed retirement retains a handle and still follows the existing failing dispatch/receipt path; this change does not pretend a failed shutdown succeeded. Regression additionally verifies no completion callback, held HTTP disconnect, unchanged encrypted pending-account bytes and no replacement upload. This closes the verified-retirement case only; transient acknowledgment failure/retry remains a separate unqualified gate. Current PR13 review audit returned zero threads with no pagination.

All82 simulator tests passc91565d17bb348ab9beeeeb91453c997; only this handoff text changed during the green run, not tested source. Unsigned device buildc1733cdbe27648ca8f2c96d3141276dd and19 harness checks pass. This fix is queued locally while previous head e8121bc CI finishes: Apple44811783469 native/device136549927313 passed3m37, simulator136549930362 still running; Android44811783482 still running. Preserve those runs rather than cancelling them with a follow-up push. Next: inspect their completion, push this reviewed fix to existing PR13, update its body and verify remote blobs.

### 2026-11-20 — reconcile a lost final-close acknowledgement

Traced ambiguous playback saves through Swift PlaybackAccountingQueue retries, private FFI receipt discard/cancellation and native CheckpointDone. The actual writer can finish after its caller is dropped; the owner was then retired and Retry Close returned stale_playback forever. CoreSession now retains only its most recent successfully closed token. Duplicate Close for that token acknowledges the original durable save without writing or touching a newer player. Still-pending close returns checkpoint_busy, failed writes remain retryable failures, unknown tokens remain stale. Logout clears the receipt and new sessions start empty. No new persisted schema or dependency.

Explicit behavior change: repeated successful Close for the retained token now returns success rather than stale_playback. Other stale progress/checkpoint assertions remain. Host checkpoint fixture runs both observed and deliberately dropped close acknowledgements while holding the real sequential writer, then verifies unchanged ciphertext, a live newer owner, reopened state and logout/new-session receipt isolation. Initial fixture compile84485a2ecee24b45979ec67729e173ff missed Some in if-let; fixed. Red1d987d114e674eb098b8d28ceef6fcbb reproduced stale_playback for both duplicate-close cases. Focused green9fa975bf688a4ea886b3773e3ec6bf14, full Core+fixture7e9d7edf5eb64babab1bc9dacbaf6fe7, expanded isolation/format final checkpoint3f98c8694b03442aad42fd2c2e794f11 pass. Swift bridge now repeats successful Close; all82 simulator tests pass93f1968e035643ba8bfba5a93646d294 (only host test formatting/docs changed during that run). Unsigned device build e1b8a065f8a14da085f930dce65f6280 and19 harness checks pass.

This closes the final-close lost-ack case only. Start/retire/progress/checkpoint ambiguity, transient reauth acknowledgment failure/retry, live services, physical lifecycle/qualification and phases6/7 remain open. Do not claim arbitrary operations replay safely. Source-independent shutdown fix40633ea is already pushed to PR13; its Apple run44813135296 jobs136554531449/136554530714 still in progress at this checkpoint, Android44813135654 and both harness checks passed. Next: await that exact run, then push this validated close fix and update PR13/body/remote blob verification. Previous e8121bc passed all six checks, simulator7m59/native3m37.

### 2026-11-20 — retry Start and Retire without replacing playback

Extended the receipt-reconciliation contract: Start for the same active/non-closing token acknowledges its existing owner and preserves clock/watched time; unrelated Start remains busy and a closing owner rejects Start. Retire retains only its latest successful token independently of Close, so repeated retirement cannot affect a newer player or be mistaken for a durable save. Logout clears both terminal receipts. No schema/dependency change.

Real Core/storage regression first reproduced duplicate Start returning playback_busy (fa97bfa1e6f546fdbb84c0cb121ede8a); after the Start fix it reproduced duplicate Retire returning stale_playback (a15c00846f41437f88f2e976ec724c44). Green54445cff44d443c18e694c0abdcd94a4. Final tests verify preserved400ms progress, unrelated-token rejection, no resurrection of retired tokens, newer-owner isolation, bounded replacement of the old retirement receipt, logout clearing and Retire-vs-Close distinction. Full Core+fixture54e5e0c7a185450e8afb22c83e2fb246,82 simulator tests032f568c93d44fce8b9bb2d30e0ac3f5 (including duplicate Start through Swift/C), unsigned device build0bdabd8ba3b04f148868a875a0c18bce and19 harness checks pass. Test/source fingerprints stayed stable throughout these runs.

These explicit lifecycle retries are safe; checkpoint/progress-operation ambiguity still needs evidence, as do transient reauth cleanup failure, live accounts, abrupt/physical lifecycle and later phase gates. Existing remote40633ea simulator136554531449 passed7m53/native1365545307144m17, Android44813135654passed4m08; aggregate was just pending when this entry was written. Ship this commit together with prior validated close fix8eea26b after aggregate completion, update PR13 and verify blobs. No other worktree files changed.

### 2026-11-20 — checkpoint and absolute-sample retry qualification

Expanded the real sequential-writer checkpoint fixture to drop both checkpoint and Close waiters while writes are held. It verifies pending retry stays busy, admitted400ms persists after release despite newer450ms player state, subsequent checkpoint captures450ms, and later failure/reopen assertions retain their independent expected values. Catalogue/player integration now repeats identical played900ms, pause/unpause and seek900ms events twice each after an800ms seek: every receipt remains time900/duration1000/watched-time100/not-watched. This proves Core's existing absolute-event behavior without changing product code or weakening assertions.

Targeted checkpoint+catalog suites passf589c0c569994bb4946105669e51d68a. Only host tests/docs changed, so device/simulator behavior inherits the previously validated af59595 code; do not claim those builds were rerun for test-only edits. The full Swift/UI FFI-timeout-and-retry journey remains unqualified even though native semantics now have direct evidence. Other live-account, transient reauth cleanup failure, abrupt/physical, earlier-stage and phase6/7 gates remain open.

Remote af59595 Apple run44814398278 jobs136558847238/136558846486 and Android44814397506 remain active; harness checks passed. Do not restart/cancel those runs. Next: finish their monitoring, push this host-test coverage to existing PR13 with remote blob verification, then continue the remaining gates. Prior lifecycle commits8eea26b/af59595 are already pushed and PR13 body updated.

### 2026-11-20 — phase 6 qualification preparation

Created isolated worktree .agent-work/phase6 on codex/tvos-phase-6 from local phase5 checkpoint157e186. Phase5 has queued host-test coverage4b23940 and corrected status157e186, neither pushed yet while af59595 Apple44814398278/Android44814397506 run. Keep that exact run intact; phase6 should be pushed/opened as a draft PR against codex/tvos-phase-5 after its base commits are uploaded. No app/native code changed in this phase6 slice.

PHASE-6.md now maps all stage6 mandatory experiments to18 named rows with exact thresholds, raw evidence/identity requirements, safe fault injection, separate controlled/public swarms, independent seeder requirement for Mac-disconnected journeys, failure/retry retention and explicit not-run physical status. Hardware model/OS remain user-reported, memory ceilings unmeasured. Current global NSAllowsArbitraryLoads is explicitly a release gate failure, not accepted as release-ready. Matrix preparation does not waive phases0–5 or claim hardware results. make check19 and whitespace checks pass; no product/device builds rerun for these docs.

Next: finish monitoring phase5 CI, push its two queued commits and verify PR13 remote blobs; commit/push this phase6 matrix and open its own draft PR with only the phase6 diff. Then implement release hardening and continue unresolved phase5/earlier software gates. Phase7 still needs its own branch/PR and release deliverables. Physical qualification/install remains deferred, so stack completion is unproven.

### 2026-11-20 — phase 7 handover preparation

Created .agent-work/phase7 on codex/tvos-phase-7 from phase6 commit7a3d41b. Replaced stale LOCAL-BUILD.md claims about an extracted snapshot/mandatory Android backend with current standalone source, pinned tool prerequisites, real Make entry points, owned fixture seeder and explicit Debug/unsigned output. Retained the dated16November baseline paragraph and SOURCE-MANIFEST provenance link; that old manifest is not a release-binary inventory. PHASE-7.md defines candidate/source/dependency/notices records, same-identity install/upgrade/renewal/removal protocol and remaining in-app cache/upload help. No physical or release success claimed.

Validated commands against apple.sh/Makefile/setup-apple and local signing example; make check19 and whitespace checks pass. These are documentation changes; no product build repeated. Phase7 remains preparation, not complete. Current af59595 CI native/device136558847238passed4m46, Android44814397506passed5m26, simulator136558846486still active. Await that run before uploading queued phase5base157e186, phase6matrix7a3d41b and phase7 preparation, opening a draft PR for each new branch and attaching the URLs. Base/heads must be phase5→phase6→phase7. Then continue release hardening, remaining phase5 software gates and actual release artifacts; physical testing remains deferred.

### 2026-11-20 — phase 6 unsigned Release build and ATS cleanup

Added make apple-release-build / apple.sh release-build: explicit Xcode Release, generic tvOS device, no signing, no simulator destination required. Wrapper boundary regression first failed with unsupported-action exit2, then passed with exact Release/device/unsigned arguments; existing combined-test assertions retained. Device CI replaces its Debug build with this Release build and checks the actual packaged plist/executable. No second device build or long routine test added. Native archives still use the existing Cargo development profile; do not call this fully optimized native Release.

Removed NSAllowsArbitraryLoads from project.yml and generated tracked Info.plist, retaining NSAllowsLocalNetworking and updating its purpose string to actual torrent/media use. Apple documentation says the local key can override the broad key on supported OS versions; this removes obsolete configuration, not proof that ATS had previously been disabled globally. Rust/VLC policies remain separate. PHASE-6.md includes primary-source links and the scope of this audit.

Unsigned Release build67e338d1e237446cae1fe2ce353c7ce9 passes. Executed the exact new CI Python snippet locally: bundle ATS dictionary is exactly {NSAllowsLocalNetworking: true}, declared executable exists. nm inspection finds no cue_core_create_account_fixture symbol in the final device executable; strings inspection finds no exact Playback Lab label. These static checks do not prove all runtime UI behavior. Full simulator82 unit/native+12 short UI tests pass400bedb0468b46a6a8adb762151e8b10, result Test-SittingRoomTV-2026.11.20_22-00-43-+0100.xcresult. During verification only workflow/tests/docs and generated plist changed; app configuration/source used by builds remained as described. Harness20 tests and focused wrapper3 tests pass.

PR15 Android44815971782 attempt1 failed before compilation resolving Maven artifacts; retained .agent-work/android-44815971782-failed.log, confirmed ASM9.6 POM HTTP200 from this Mac, retried failed job unchanged (attempt2 job136565742260 still running). PR16 Android44815974592 failed with explicit Maven429 rate limits; retained .agent-work/android-44815974592-failed.log. Wait for PR15 retry before retrying PR16 to avoid another simultaneous dependency burst. Existing Apple jobs for PR13/15/16 remain active; do not replace or restart solely because of elapsed time.

This phase6 commit still needs push after current PR15 run finishes, PR15 title/body update and remote blob verification, then merge into phase7 and update its build guide to include unsigned Release. Phase7 remains at7e7ba86/PR16. Root dirty CI/HANDOFF files remain untouched. Physical Q18 and the broader qualification matrix remain unrun; no phase marked complete.

### 2026-11-20 — phase 7 incorporates Release build audit

Merged phase6 commit97c2fd6 into phase7 as9b02dcf, preserving both appended handoff entries and combining the phase6 build status with phase7 preparation in the two documentation conflicts. Updated LOCAL-BUILD.md/PHASE-7.md to include make apple-release-build and accurately distinguish unsigned Xcode Release, existing Cargo development-profile native archives, packaged ATS verification and still-unqualified signing/runtime. All20 harness tests passed after the merge. No product source differs from validated phase6; product builds are inherited, not claimed rerun.

Queued remote updates remain: phase6 at97c2fd6 and this phase7 merge/docs after existing runs complete. Current phase7 base should stay codex/tvos-phase-6, PR16; PR15 should be retitled/described for the actual Release build/audit implementation, not only the matrix. Still required in phase7: actual build/source/dependency manifest tied to its binary, notices/source materials, app help and physical install/upgrade/renewal/removal. Do not treat these docs as final release evidence.


### 2026-11-20 — phase 7 in-app help

Home now opens a remote-accessible Help screen explaining watch/resume, current 4 GiB cache/1 GiB reserve, peer sharing and account recovery. Menu dismisses and restores Home focus. The opt-in HelpJourneyTests journey checks navigation and sections without adding a long routine CI test. First behavioral pass05fc913061c54a08bac171dca94185bd exposed a transparent sheet during screenshot review; an opaque background fixes the visible Home bleed-through. Final screenshot .agent-work/help-ui-opaque-evidence/59AE91F6-E587-4EBC-A6BC-2F4AB3D0AECD.png reviewed: all text fits, no bleed-through, Close help focused.

Help and existing sign-in cancellation journeys both pass9d9e96b1906a40f98d30ffa51ca657d8 with stable source fingerprint (19.044 seconds total). Final unsigned Release passes4d9d3afd3d084ab6a67c6aba909ffb88; only documentation changed during that build. Earlier e1dcadbabff847a4b1bb7b8809737032 failed before generation because this shell lacked pinned XcodeGen on PATH; retained failure and reran with repository toolchain. Harness20 passes. Physical readability/VoiceOver and final device-qualified guidance remain unverified.

PR15 current remote checks all pass. PR16 Android retry and native/device now pass; simulator job136564277196 remains live. Phase6 Release build commit97c2fd6 and phase7 updates remain queued for push after that run completes. Preserve root dirty documents. Final source/binary manifest, notices, remaining earlier software gates and physical qualification still prevent merge-readiness.


### 2026-11-20 — Release fixture packaging

Actual Release inventory exposed all17 media/torrent/manifest fixture resources despite the Debug-only lab entry point. Added a packaged-app assertion to the existing device CI lane; it first failed on all17 filenames. Release-only EXCLUDED_SOURCE_FILE_NAMES now omits those resources. Final unsigned build6ea24870260248e881751e70574f97c1 passes with stable source identity; executed the exact CI ATS/executable/resource audit locally, passing. Debug retains all17 files with SHA-256 matching their sources. Standard simulator suite646110cff2c34ee8915170a84dbad818 passes; only PHASE-7 documentation changed during this run. Harness20 and diff checks pass. No new long test, dependency or second device build.

Phase6 commit97c2fd6 is pushed on PR15 and all10 changed remote blobs verified; its new CI is running. PR16 old7e7ba86 checks all pass, including simulator136564277196 in21m06 (test/build step13m31; remaining time includes setup/cache). No current review threads exist on implementation PRs3/4/5/6/7/8/13/15/16; review-thread pagination was exhausted. Publish phase7 Help plus this packaging fix next and update PR16 from .agent-work/pr16-update.json; retain draft status. Final source/binary manifest, complete notices/source materials, earlier software gates and physical qualification are still outstanding.


### 2026-11-20 — unsigned candidate recorder

Added release-record.py and3 focused inventory/identity tests (23 harness checks pass). It requires a clean pinned checkout, records tool versions, executes make apple-release-build, retains failure receipts/logs, verifies unsigned state, rejects source/private-input drift, copies3 lockfiles and inventories app files/native archives. Private config bytes are not copied; hashes stay local. It rejects symlinks rather than silently following unrecorded dependencies. Incremental provenance is explicit; independent clean-build repeatability and license qualification are not claimed.

Candidate .agent-work/release-candidate-001 succeeded at clean7ee7493 with stable task identity1651cc4f10454521a5ca5abb91c250ce. Records Xcode26.6/SDK26.5/Rust1.100.0/XcodeGen2.46.0,7 packaged files,2 native archive hashes and3 resolution files; independent rehash matched all7 packaged files. No signed/physical release claimed. Keep local private-input evidence out of shared reports.

PR16 native job136576198353 failed before device compilation in checkpoint.rs:31 with AlreadyExists. Raw log retained .agent-work/ci-136576198353.log. Two concurrent tests used PID+timestamp directory names; coarse/equal timestamps can collide. Fix underway in phase5 using an atomic per-process sequence, preserving every checkpoint assertion. Propagate that tested fix through phase6 andphase7 before next push. PR15 and PR16 other live jobs must be observed, not blindly restarted.


Checkpoint directory fix now committed4196038 on phase5 and merged into phases6/7. Both concurrent checkpoint journeys passd2fabc17e5b54c68872c2c2d0ba8fd15 with unchanged persistence assertions. The first local commandb9e89503eda7498f92a1ccc2c4899cd2 omitted shared target/debug settings, created a separate1.4GiB target and failed linking with no space; deleted only that newly generated native/cue-core/target and reran with CARGO_TARGET_DIR pointing to the shared cache, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_INCREMENTAL=0. Remaining disk space is about932MiB; avoid redundant output trees. No production Rust code changed, so device/simulator product evidence remains applicable.

Phase6 remote97c2fd6 is fully green (simulator7m47/native6m36/Android4m25). Phase7 remote100dfa8 has the known checkpoint failure; its remaining jobs were still live. The new tested fix warrants publishing all three updated stack branches; do not retry the defective head unchanged. Candidate recorder commit7ee7493 and its docs also await publication on phase7. Full software/device gate audit remains incomplete.


### 2026-11-20 — packaged VLCKit source provenance audit

Inspected actual candidate framework and pinned wrapper, retained the exact wrapper git archive (1,601,146 bytes) under release-candidate-001/vlckit-build-materials, and recorded its SHA-256, packaged framework hash, declared download checksum, build script hash and all 12 patch hashes in evidence/vlckit-provenance-2026-11-20.json. Independently read the archive and verified script/patch contents. The wrapper's TESTEDHASH resolves through the official VideoLAN GitHub mirror to f7caf7922478f6cb7f1654b813d0c462d3e86f4a. The framework contains 4.0.0-dev-39728-gd107c01fbc; that patched changeset is not resolved to the published build's exact source/configuration. Complete contrib source/notices and independent rebuild remain open, not waived by generic LGPL text.

Offline locked tvOS Cargo metadata retained locally: 209 reachable Core and 275 torrent packages including build edges, excluding dev edges. Missing metadata declarations are cue-core/localsearch/stremio-core/stremio-derive/stremio-watched-bitfield and cue-torrent; match their actual source notices rather than inventing licenses. This count is not exact binary linkage evidence. Harness23 passes; docs/provenance only, no app rebuild needed. Current remote phase13/15/16 corrected heads are running; PR15 aggregate failure44820187480 belongs to a cancelled duplicate event, with replacement44820187863 still live. Do not rerun that cancelled revision blindly. No unresolved review threads as of previous audit; stack remains incomplete.


### 2026-11-20 — native source-notice correspondence

Mapped all six packages missing Cargo license declarations to existing repository/source notices; no licenses invented or upstream metadata changed. PROVENANCE.json records 17 package-to-notice mappings including rqbit workspace crates and source revisions/checksums. Copied rqbit's root Apache notice verbatim. Stremio Core's retained MIT differs from source only by its final blank line, now normalized for byte-exact evidence; local-search already matches. Neither this map nor SPDX declarations establish complete combined-app notices.

The combined non-dev/build graph has 327 packages. Initial top-level/explicit discovery leaves fxhash0.2.1, stremio-official-addons2.1.2, assert_cfg0.1.0 and governor0.10.4 needing additional materials; fxhash has source header, Stremio README references omitted LICENSE.md, assert_cfg VCS4799cdc4e5e1a94ef6bdb24d8c5e7a71c72e2601 and governor VCS9f3a79dd47dd32acd589c562b8d4fefe99b93372 identify exact upstream source. Retain all remaining transitive notices and finish VLCKit contrib/source evidence before claiming completion. Changes are notices/docs only; product behavior unchanged. Current corrected PR runs remain live; no blind reruns.


Follow-up recovered exact upstream assert_cfg/governor license blobs using their recorded VCS revisions, Stremio official-addons LICENSE.md at v2.1.2 resolved commit272e36095d6ece7aea97f094a02a4fb0a94def59, and fxhash source copyright/license header at537570af68de22d2f4bbdd5bee5d6e07268e6f42. Stremio and fxhash lib.rs blobs match the downloaded crate files. Git blob SHA-1 was independently checked for the three retrieved license files; all retained notice SHA-256 values rechecked. Retained full Apache2.0 text from its official URL for fxhash's permitted alternative and rqbit, whose root LICENSE is only an attribution notice. PROVENANCE.json now maps21 packages plus standard text. Harness23 passes; no product build rerun for these notice/docs additions. Full327-package notice collection, exact VLCKit bundled-component/source correspondence and clean-build/physical gates remain open. Current CI jobs remain active on corrected heads. Both this notice commit and preceding a03e8be provenance commit should be pushed together after the current phase7 run finishes, avoiding needless cancellation.


### 2026-11-20 — locked native notice collector

Added collect-native-notices.py: offline locked tvOS Cargo resolution, graph traversal including build dependencies and excluding dev-only branches, source notice retention deduplicated by SHA-256, and reviewed overrides gated on exact version/resolved source/checksum. Missing or stale mappings produce nonzero outcome with an explicit missing list. Manifest records source commit/dirty state, collector/provenance/lock hashes and scope limits. Top-level/explicit notices do not prove complete nested third-party or VLCKit coverage, nor exact binary linkage.

Four regression seams cover graph filtering, missing/version-mismatched notices, modified notice checksums and same-version Git revision changes. Initial missing-collector failure retained .agent-work/native-notices-red.log; final harness27 passes1ff3c4ba5c734cf89959065be7f5b186. Final collection49b984402e9f4b7fb5145d389ca369bb at .agent-work/native-notices-002 retains231 unique texts for all327 packages; independently verified every referenced text checksum and no missing entries. Earlier001 records the initial implementation;002 includes collector/dirty-state metadata. Product untouched, no device/simulator rerun needed.

Current corrected phase6 checks all pass (sim8m54/native4m31/Android4m30); phase5 andphase7 native+Android pass, simulator jobs136578768181/136579072495 remain active. Notice/provenance/collector commits remain queued until phase7 current run ends. Source-preserving license copies intentionally retain upstream final blank lines; generic git diff whitespace check flags those, not modified license text. Still open: nested notice audit, VLCKit component/source matching, independent clean-build evidence, earlier software gates and physical qualification. Preserve low disk space and root dirty documents.


### 2026-11-20 — nested notices and combined candidate evidence

Collector regression first failed because third_party/codec/COPYRIGHT and LICENSES/MIT.txt were omitted; raw red .agent-work/nested-notices-red.log. Recursive filename/license-directory discovery now includes them, excludes generated/cache directories, rejects discovered notice-file symlinks, and retains relative source paths. Current native-notices-003/e4869ddb225d470ebe18dfc59e8985b0 includes244 unique texts for327 packages:13 additional texts include AWS-LC/fiat, ring/once_cell, Unicode and tracing notices. Every text hash independently checked. Embedded source headers and complete VLCKit component/source matching still need review; do not call this license compliance.

Release recorder now runs the collector and hashes its manifest into the candidate record; collection failure prevents final candidate issuance. Harness28 passes9bb0a450dd0743fca27eafd3564aa338. Real unsigned candidate002 passes498621eead8549358e27af261f1682ee at clean9170965 with stable source identity. Independently verified notice-manifest hash, matching clean source commits,327 packages/no missing entries and every notice text checksum. No product behavior changes or extra simulator run. CI phase5/7 simulator jobs remain active, phase6 corrected head all green; queued notice/collector commits should publish after phase7 existing run finishes.

Disk917MiB free. Shared native-target totals device10GiB/simulator9.7GiB/host7.3GiB. Device and simulator incremental directories alone are1.1GiB each, regenerable caches; preserve source, evidence and built artifacts if cleanup is needed. Independent clean rebuild still pending. Before claiming reproducibility, capture build-affecting profile environment settings in candidate records; current recorder captures tools/command but not all environment. No physical gates waived, earlier software gates remain.


### 2026-11-20 — fresh-output unsigned build and main conflict

Added common compiler/profile environment capture with an explicit allowlist; regression first failed missing build_environment, then harness29 passes91b810cb980444329b74e53a280a4c25, retaining .agent-work/release-environment-red.log. Does not dump unrelated credentials; arbitrary custom tool configs remain outside this claim.

With no compiler processes active, removed only regenerable device/simulator incremental caches. In phase7, preserved old native-target/DerivedData symlinks as native-target-before-clean-rebuild and DerivedData-before-clean-rebuild, then created fresh directories; only SourcePackages points to the previous pinned download/artifact cache. Phase7 now uses its own fresh outputs, NOT the shared output symlinks. Other worktrees still use the shared cache. Fresh candidate clean-001 passedfa705e57dc044422a2be9a475f2e120c at clean dbcc61e with unchanged source identity, using DEV/TEST_DEBUG=0, INCREMENTAL=0, BUILD_JOBS=4 and CARGO_TARGET_DIR pointing to phase7's fresh native-target. Both native libraries and app compiled successfully; exact CI ATS/executable/fixture audit and notice hash verification pass. Prebuilt pinned VLCKit was reused, not source rebuilt. Physical/runtime and remaining earlier gates remain unverified.

Review audit .agent-work/stack-review-audit.json exhausts review bodies, comments and threads for PR3/4/5/6/7/8/13/15/16: none outstanding. Mergeability audit found PR3 conflicts with main755f38f (merged Apple CI PR11). Clean plan-ci checkout was six commits behind remote; fast-forwarded to6deedaf, merged main, and resolved four CI/harness conflicts preserving main's stage-aware pinned workflow/apple.sh and all harness Make/ignore entries.13 checks pass; committed3df82b5 locally. Not yet pushed. Propagate this main integration through phases0–7, reviewing each stage's workflow/command tests. Root phase4 checkout has preserved dirty docs: DO NOT merge into/reset/edit that checkout. Use a detached phase4 integration worktree and push its resulting HEAD to the remote phase4 branch; merge that new commit downstream while leaving root local branch/index untouched. Existing helpers inactive. Current phase7 a89f8e6 CI all green (sim10m47/native7m16/Android4m47); queued local phase7 commits can publish together after stack integration. Do not mark drafts ready or goal complete.


### 2026-11-20 — main integration propagated through stack

Main755f38f now reaches all local phase heads through plan3df82b5, phase0 9bc12bc, phase1 2286ee5, phase2 merge, phase3 fcc2f09, detached phase4 919721d6fa246260d3b9450dc4f84db25d685a3f, and phase5/6/7 merges. Verified each base is an ancestor of its child; .agent-work/main-integration-chain.json records hashes. Phases3–7 merges preserve their entire previous trees. Phase3 conflict resolution retains its newer exact-artifact cache, mandatory Core/torrent commands and mandatory combined UI/unit selection; the baseline's missing-component guards/test apply only to pre-native stages. Harness checks: plan/0/1/2 each13, phase3 15; later unchanged trees inherit current green product evidence. Root phase4 branch/index and its two dirty docs remain untouched. Push phase4 using the detached integration commit as source, not the stale local phase4 branch.

Before this upload, corrected PR13/15/16 all green: PR13sim15m24/native9m09/Android4m33, PR15sim8m54/native4m31/Android4m30, PR16sim10m47/native7m16/Android4m47. Their previous checkpoint collision is resolved. Push the full stack atomically, verify branch/tree identities, update PR16 for its accumulated release evidence tooling, and monitor new checks. No merge to main and no ready-for-review promotion yet: physical/earlier software/VLCKit source gates remain open.
### 2026-11-20 — retain refresh ownership after lost cleanup receipts

Added accountRefreshRetainsOwnershipAfterLostCancellationReceipts using real synthetic HTTP, NativeCoreService/FFI and encrypted storage. A forwarding CoreService boundary drops the first two cancellation receipts only after native cancellation succeeds. The test checks retained sequence ownership, disabled resubmission, unchanged pending ciphertext, retry-Cancel recovery and eventual same-account upload without replacing the session/library. Production code is unchanged.

Initial harness117809a9b8c1455eaa23586b2971b02a raced cancellation before admission and failed to exercise the intended boundary; corrected by waiting for wrong-account rejection and automatic cleanup. Focused green d02369f7765d4a9a8771acf32b559705 passed one test. Mutation8d88c4fb2a0440af979b8129f569519c moved sequence release before cancellation acknowledgment and failed six intended assertions. The production file was restored byte-for-byte; final baseline f15b3366a160445e90d8519591369123 passed exactly one test in1.016s with stable source fingerprints. This is controlled receipt loss, not an actual native queue-full/timeout fault or UI/physical qualification. No claim that the entire simulator suite was rerun.

Remaining gates include actual native timeout faults, complete Swift/UI operation reconciliation, live services, abrupt/physical lifecycle and earlier/later phase requirements. Carry this test through phases6/7 after review; retain current CI runs rather than restarting them for this test-only addition.


### 2026-11-20 — recover an unreadable account selection

A malformed Keychain selection previously left startup failing with neither a usable account nor a sign-out path. Added explicit confirmation in Home and a model recovery action. CoreAccountSelection revalidates synchronously on its shared actor before deleting only an invalid selection record. A valid/newer selection or Keychain access failure is not cleared. No account directory is inferred from corrupt bytes, and account files/keys remain untouched. The confirmation explains that a new login will not restore retained unsynced changes. This is selection recovery, not orphan-data reclamation or automatic data repair.

Regression16d646f743cc4f03b27de1972c654379 failed the absent recovery availability/action; focused85cccfb037ed4a119f50cfe39f252343 passed. Extended coverage retains an actual account Keychain key and sentinel state.bin bytes and rejects a valid replacement selection installed after the error. Full84-test run1a906bf3b050468fbe3b237e6019f27f failed five torrent assertions while disk had only about400MiB free, below the1GiB reserve. With no builds active, removed only obsolete generated device cue-core compiler output directories ccf03a38c73c59ef and2572477c94736837, preserving source, final archives and evidence. Free space rose above3GiB. Identical full84-test suite18150dd7439c4949970e5dd84ab15d55 then passed; this isolates those failures to environment capacity. Harness19 passes. No native/dependency changes; new confirmation-dialog navigation/visual and device build are not claimed verified.

Earlier cancellation-receipt regression is committed a92902a, propagated locally to phase6 f07eb00 and phase7 77fecbe. Those updates and this recovery slice remain queued while current remote Apple workflows run. Continue UI/device verification, propagate the final commit into phases6/7, then publish to existing PRs13/15/16 after current checks finish. The stack remains incomplete: live services, actual timeout faults, abrupt/physical lifecycle, earlier software and final release gates remain required.


### 2026-11-20 — selection recovery UI and device compilation

Extended the simulator-only account fixture with an isolated malformed-selection namespace and teardown that can remove that malformed fixture record. New opt-in AccountJourneyTests test exercises actual Home confirmation, Cancel preserving the error, Reset returning to the guest library, sign-in availability and empty-selection teardown. No server is required for this case; existing account runner counters remain unchanged. Routine CI still selects only unit/native tests and PlaybackLabTests.

First UI run f1c312c79d7746568f7ac56bcfa4425c failed because tvOS exposes nested duplicate accessibility buttons. Captured hierarchy shows the outer Cancel button focused and horizontal buttons. Selecting firstMatch and navigating horizontally fixes test targeting; production UI unchanged. Final c47630f3e7b84b7cb7b25b2d0730b57e passes exactly one UI test in20.089s. Screenshot .agent-work/invalid-selection-ui-green/1403D1C2-0289-41FF-8DE1-434075902D41.png visually reviewed: complete text, clear safe default focus, no clipping. Unsigned device build c5b040b41ed44068ab0bef1dcba8e890 passes with stable source fingerprint; harness19 passes. Previous full84 unit/native suite18150dd7439c4949970e5dd84ab15d55 remains current for production source; only fixture/test/docs changed afterward.

All stack PRs are mergeable with zero review threads/reviews in the current audit. Some historical cancelled duplicate runs still appear failed; use current PR checks rather than treating an all-history status rollup as current failure. PR13 native/device136589410601 passes6m34; simulator136589410048 is still running. PR16 replacement Apple44823272817 also remains active. Queued phase5 changes must be merged through6/7 and uploaded after those runs; pending PR13 body is .agent-work/pr13-recovery-update.json. Physical qualification and other outstanding requirements remain open; do not mark the stack ready.


### 2026-11-20 — cancel obsolete catalogue HTTP and permit identical-query retry

Real-socket regression0735663740b3453190b3ba0419475466 reproduced obsolete search HTTP remaining open after a new accepted search. Stale-result rejection alone had not cancelled transport; identical query reselection could also retain Core's old loading state. Added an environment-owned catalogue cancellation token captured when the add-on resource effect is constructed. Accepted catalogue changes and logout cancel the previous token; only catalogue resource futures observe it. The stale effect still settles through normal actor receipt accounting. Reset the catalogue model on explicit reselection so identical queries fetch afresh. Account, metadata, source and subtitle requests are not part of this cancellation scope. No new dependency or lock change.

Catalogue5-test suite39a1d70db3c34d6db14a0b62afbedd9c passes; full Core+account-fixture suiteb3eac1d69f4c432695ad25ead5d7b935 passes. Removed unrelated rustfmt-only changes afterward. Simulator17b1ecf0dd404c4f945985f6a035b4b7 passes all84 tests in35.760s. Unsigned device buildcb1c69d9d53447a28b50bbcdb6c4d1a4 passes; only a redundant blank line was removed before that build. Final focused catalogue run additionally bounds fixture admission/server waits so failures do not hang CI. Rust1.100.0 used throughout; cargo fmt without an explicit toolchain initially reported missing rustfmt on the stable alias, then pinned rustfmt succeeded. Harness19 and whitespace checks pass. Preserved raw failures and evidence; no test assertion relaxed.

For disk headroom, after all local builds stopped, removed only obsolete17-Nov simulator cue-core generated output directories28a2ad145d129311 andfc0eb718c80a7be3. Source, archives, evidence and root dirty documents remain intact. Current remote PR7 is fully green: simulator14m55/device9m03; PR13 also green at its preceding head. PR16 device9m18 passed, simulator136591643088 still active. Queue this change with the previously validated selection/receipt fixes, propagate to6/7, then upload after the current phase7 run finishes. Remaining transport scope (obsolete metadata/source work), retention budgets, live/physical and release gates remain unqualified.
