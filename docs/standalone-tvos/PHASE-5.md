# Phase 5 — resume, library and recovery

Status as of 20 November 2026: implementation is in progress; this phase is not
merge-ready. The standalone app has acknowledged library/watched saves, five-second
playback checkpoints, Resume/Restart, an encrypted bounded sync journal with capped
retry and same-account reauthentication, and local playback diagnostics. Controlled
simulator playback and ten graceful exit/relaunch cycles have passed. These do not
establish abrupt termination, physical suspension or live-account qualification.

The contract is [decision 003](decisions/003-resume-library-sync.md); the full exit
gates remain in [stage 5 of the plan](IMPLEMENTATION-PLAN.md#stage-5--resume-library-and-recovery).
Branch: `codex/tvos-phase-5`, stacked on `codex/tvos-phase-4`.

Current simulator evidence includes 81 unit/native tests plus 12 short UI tests
(`abe31d8c80c141d3902b1741dd67084e`), two opt-in account UI journeys
(`0a22a1b23881460880561bc6209fecdc`), and unsigned device compilation
(`7a55ec9fb52f40fb8d6b859e5e16a0d2`). Native Core+fixture tests passed
`1f2c3c0a56434d2f95ad6196bf11c476`; no native implementation changed since.
These task records live locally under `.agent-work/tasks/phase-5-library/`;
they are not committed evidence artifacts. The committed
[resume-cycle report](evidence/resume-cycles-2026-11-18.json) records the graceful
relaunch experiment. All six remote checks passed at checkpoint `e96a5f7`;
that result precedes the latest first-sign-in UI additions.

Synthetic-account tests cover same-account refresh after HTTP401, wrong-account
rejection, encrypted-save failure and held-login cancellation, each with safe retry.
The cancellation receipt follows publication of its settled snapshot, fixing
immediate retry against a stale active flag. Remote-driven simulator UI now also
qualifies first sign-in, rejected-password retry, account adoption/catalog browsing,
reauthentication with retained library content, and sign-out. These exercise real
HTTP, FFI and storage against an isolated synthetic service, not live Stremio.

Remaining work includes Swift ownership/retry after failed reauthentication cleanup,
live-service qualification, ambiguous operation timeout reconciliation, full failed-save UI and abrupt interruption
coverage, physical cache-purge recovery, and broader console privacy qualification.
Simulator composites now qualify pending-journal sign-out with held HTTP and failed
filesystem cleanup/retry, plus repeated peer transfer after disposable-cache deletion
while encrypted account state and keys remain intact.
Earlier phases' unresolved gates and the physical-device matrix remain required.
Rust panic-hook redaction and VLC logging suppression are implemented; neither
proves that all OS crash reports or third-party output are private.

The sections below are chronological implementation records. Their statements of
“next” work or missing APIs describe the date of each slice, not the current state.
Use the latest [handoff entries](../agent-harness/HANDOFF.md) for subsequent evidence.

## Opt-in account-recovery UI journey

From the repository root, with the pinned build toolchain installed and an available
tvOS simulator, run:

```sh
bash scripts/fixtures/check-account-journey.sh \
  'platform=tvOS Simulator,id=YOUR_SIMULATOR_UDID'
```

The runner owns a loopback server on port 42429 and stops it on exit. It waits for
that process's readiness log before launching Xcode. Two remote-driven UI tests
use isolated synthetic accounts: Guest → rejected password → successful first
sign-in → catalog browsing → sign-out; and rejected pending library upload →
rejected refresh password → successful refresh → retained library → sign-out.
Both check that rejection clears the password. The server independently asserts
three accepted logins, two rejected logins, one rejected upload and one accepted
upload. No live account service or credentials are used.

XCTest teardown cleans each opaque storage namespace/Keychain selection in a
separate launch, including after fatal UI-query failures. Successful journeys also
require no leftover selection or account directories before cleanup, so cleanup
cannot conceal failed sign-out. Test views and fixture transports are absent from
physical-device builds. These journeys stay outside routine CI and do not prove
live-service compatibility or physical remote behavior. Captured catalog/library
screenshots were visually reviewed for readable content and focus; local paths
and exact results are in the handoff.

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

## Native save-receipt interface

`cue_core_request_library(handle, member, revision)` uses the existing exclusive
ticket slot; member is strictly 0 or 1. The queued operation admits the guarded
Core action, waits for settled effects, rejects any Core error or mismatched
membership, then returns a versioned receipt. It has a four-second bound. Discard
cancels observation, not an accepted mutation; callers must refresh state after
cancellation/timeout. Stop still retires the native owner and discards replies.

Missing-symbol red: `c5c634d395ea416cb56fc8397a6a1c4e`. All five FFI tests pass
(`bcbf7ccfd4ee430589e43eec24336ead`), including invalid boolean, exclusive-slot
admission and a no-selected-title failure with matching session/ticket and no
success payload. The first build attempt failed from local disk exhaustion
(`7c0ce8ae460d497eae1c5e486f26589d`); package-scoped Cargo clean removed only
generated Cue Core output before the successful rebuild. All 15 portable checks
pass. Successful save receipts through Swift and interrupted saves remain to
be exercised; the previous native persistence tests do not establish that bridge
coverage. No Swift-facing API or UI is added in this slice.

## Swift save receipts

CoreService now exposes `setDetailsLibrary(_:expectedRevision:)`; NativeCoreService
uses the bounded native ticket, checks cancellation/owner retirement, validates
version/session/ticket through the existing reply decoder and verifies receipt
membership matches the requested value. Snapshot decoding adds optional selected
membership without requiring old fixtures to provide it.

The real simulator HTTP fixture now adds the loaded title, observes membership,
rejects removal with the old revision, then removes it with the current revision.
Missing Swift API red: `cfae6f0fb7e6405fbccab89e26dac105`. All 60 Swift tests pass
(`7819d142aaba44e48f7621119a44c4f7`), including successful receipts across the
Swift/C/native boundary. All 15 portable checks pass. The native encrypted reopen
and disk-failure tests remain separate evidence; this Swift fixture is in-memory.
User-facing library controls and interrupted save coverage remain outstanding.

Unsigned tvOS device build passes (`8ab87378acaf48be88a478f356765a00`); only
this evidence document changed during that build, not source or tests.

## Title-detail library control

Ready title details now offer Add to library / Remove from library. The control
keeps remote focus while saving and ignores repeated presses, waits for the save
receipt, and retains a failed intent behind Retry library save. The task belongs
to the title view; a cancelled/disappeared view does not publish a completion.
Cancellation is not a rollback. This is not yet a library-browsing screen.

The owned remote journey initially fails for the missing control
(`2baceb74f1d849ccadb989771a0544d3`), then adds and removes the film, chooses its
source and verifies Back focus (`b8b5344c99d24adaafdb2c819652f8b8`). Unsigned device
build passes (`1b8bf4c72fb84d81a6e575e1d302c58b`), as do all 15 portable checks.
UI-level failed-save retry and disappearance during a delayed receipt still need
controlled coverage; native write-failure tests do not prove those UI states.

## Pending receipt cancellation

The real HTTP fixture now holds a manifest response open while a library mutation
has been admitted. A concurrent handoff reports busy; cancelling the save releases
the serial bridge owner, allows a following language command within two seconds,
and refreshes the accepted membership rather than pretending it rolled back. Stop
closes the held socket. Initial green: `5a07a584077242169d94418f3f5a7f14`.
Temporarily disabling the native cancellation arm causes the following-command
and refreshed-state assertions to fail (`a13006ff3a7c4438a23cc5edcf9c643a`). The
production source was restored byte-for-byte. The earlier compile failure
`6837c60fed4e4117be8d731f76d7d9d2` was an await-in-autoclosure test error, not
a product defect. This proves service-level cancellation; view disappearance and
UI failed-save retry still need their own controlled tests.

Final restored-source run: all 60 Swift tests pass
(`e20236255b054007845c0b862068b558`). All 15 portable checks pass.

## Native library browsing

Snapshots now include a 24-item library window, offset and total. Only permanent,
non-removed members with admissible identity appear. Ordering is deterministic by
name, type and ID; names are bounded and control characters removed. When a library
shrinks, both rendering and selection clamp to the same last valid window.

CoreSession now supports revision-guarded page changes and library-title selection.
Selection reuses the existing metadata loading/generation-retirement path without
starting source discovery. Tests cover 50 members across three pages, removed/temp
entries, invalid IDs, empty/shrinking windows, out-of-range indexes and stale
revision rejection through the real session. Missing-module red:
`a62a23ac39a84976946acfd5a2e6e0e5`; an intermediate fixture omitted required Core
state fields (`5a6879a5d8584a589578d4820b56ff7b`) and was corrected rather than
changing production decoding. Full native suite passes:
`4cb6534b19ef4ab19804f8db1ea7d69d`.

These browsing commands are not yet exposed through FFI/Swift or a library screen.
The seeded session test proves selected identity without add-ons, not successful
remote metadata retrieval; the next UI fixture must cover that complete route.

## My library screen

Library paging and title selection now cross the JSON command boundary into Swift.
The catalog home offers My library, with previous/more controls, an empty state and
stable title focus. Details remember their catalog/library origin so Back returns
to the correct list; opening a saved title fetches metadata through the same Core
path as catalog browsing.

The owned UI journey adds a film, leaves details, opens it from My library, removes
it and verifies the empty list plus home focus. Missing-screen red:
`46974dd7d6b548469d19c06234db73e3`; an intermediate test allowed only ten upward
moves despite the larger default catalog list (`3d1823d34d6849d48ca6e65a3ab81e10`).
The corrected bound matches the existing twenty-move catalog traversal and adds
a restored-catalog-focus assertion. Final journey passes:
`52507771e08e46248251a96b6ddf57a1`. All five native bridge tests pass
(`794548f246eb4d399ed5c4770d2554d2`) and all 60 Swift tests pass
(`c252449f230449afa722a04889671df4`). Multi-page native identity coverage is
recorded above; a multi-page remote UI fixture is not yet included.

PR13 native job134355404969 exposed a separate fixture collision: parallel tests
used the same process ID/timestamp directory. Names now include an atomic sequence;
create_dir still rejects collisions rather than sharing state. All three library
tests pass with eight test workers (`ff0e7109cec34c328324eac0e40fbb08`). No test
was serialized or assertion weakened.

Unsigned device build passes (`48202301d48a42afbd5a2aa89ab0d36e`); all 15
portable checks pass.

## Watched state and controls

Film completion and episode bitfields are exposed only for matching selected
metadata. Core actions own watched mutations; same-state retries explicitly flush
storage without incrementing a film's completed-viewing count. Tests inject two
failed writes, verify recovery and unchanged membership, inspect disk before any
subsequent membership write, and reopen encrypted storage. Disabling idempotence
produces count4 instead of1 (`06778f90da2b44b5898af63bbb078ba3`); restored native
suite passes (`5fdcd20735474fb8b989e4f8a0a266c1`), with later disk assertions passing
in `a6583a2088f34191a59a5a0e31f7430a`.

The bounded C/Swift ticket carries watched/video identity, awaits settled effects,
and supports cancellation without claiming rollback. All60 Swift tests pass with
real HTTP watched/unwatched/stale requests and held-effect cancellation
(`0e2c7a4106f6443e86b6a31cc9bbe5b4`); full native suite passes
(`72a81dd0ccee4d96965b76175f357f78`).

Film and per-episode controls retain failed intent for retry and cancel observation
on disappearance. Remote film mark→return→unmark→source/Back passes
(`354830196b0d44a6a62d13f350a9cb0e`; missing-control red
`9b6e0219cecd4d7d98e856ba137756f4`). Episode mark/unmark leaves the neighboring
episode untouched and preserves Back focus (`e504b9ef1c61444b8b9b840baa040dca`).
The new column initially broke season navigation (`68e781ea964240f8a51e6c6d6cb65bc3`);
explicit first-episode focus on season changes fixes the unchanged test
(`62d962ace60a44ddb7401af43b86aa8a`). Unsigned device build passes
(`957d96d4b98c422fa0355881f9e78c02`), as do15 portable checks.

These checks do not establish UI failure/disappearance recovery, playback manual
override precedence, resume checkpoints, offline reconciliation or physical-device
behavior. The longer remote journeys remain opt-in rather than routine CI.

The existing add→library→reopen→remove remote journey still passes with the new
watched controls (`7b3a5299aca74692a61c3b807d3da1e7`). Episode control identity
includes type/title/video so a reused video ID cannot retain another title's save
intent. Dedicated failed-save/disappearance UI coverage remains outstanding.

## Add-on activity reporting

The no-telemetry audit found Core can send library/watched and player activity to
add-ons advertising those resources. NativeEnv now rejects those activity resources
locally, without forwarding viewing identifiers. A real HTTP test reproduces the
previous request and verifies that only manifest/catalog content requests reach
the server after the fix. Library mutation tests advertise both event resources
and still pass, including encrypted persistence and retry. Full native tests,
60 Swift tests, unsigned tvOS build and15 harness tests pass; records are in HANDOFF.
This is scoped to add-on activity reporting. Direct authenticated Player rating and
premium intro/outro effects remain under review before Player progress is enabled.

## Native Core Player accounting

CoreSession owns one Core Player behind a single-use prepared-source token. The
native API distinguishes seek, played progress and pause; it rejects retired-owner
callbacks and a second active owner. Existing ready metadata initializes the model.
Core's accounting rules produce the library item; automatic Player effects are not
executed. Content retrieval remains with the established source/subtitle owners,
and durability must use the forthcoming acknowledged checkpoint path.

Real-session tests cover strict >70% watched-time completion, seeking without
viewing credit, pause, unknown duration, backward-position rejection, clamping,
owner replacement and stale callbacks. A mutation that routes Seek as TimeChanged
fails with false completion; restored native tests and the unsigned tvOS build pass.
Evidence is recorded in HANDOFF. These APIs are not wired to Swift playback yet;
manual override, local checkpoint persistence, resume UI and lifecycle barriers
remain incomplete. Retiring this accounting model currently makes no durability claim.

## Manual intent during native playback

Matching film/episode manual changes override automatic completion for the current
Player owner, including when the details model already reports the desired value.
Decoder position remains the progress baseline, so marking watched cannot count the
same viewing interval twice. Other title/episode edits are ignored by that owner.
Explicit new Play permits automatic completion again; initialization alone does not
change the manual state. Tests cover these boundaries with the actual Core models.

Duration updates at an unchanged clock now synchronize without adding watched time,
so unknown duration cannot accidentally use a prior known duration for completion.
The full native suite and device build pass. These native rules are not yet connected
to Swift playback callbacks or durable checkpoint projection; those gates remain open.

## Atomic local storage prerequisite

NativeEnv now supports replacing several serialized values with one encrypted-store
commit. Memory changes only after persistence succeeds; the combined final store
must fit the existing storage budget. Single-key Core writes use the same path.
The failure/reopen test verifies that both old values survive a failed write, both
retry values survive reopening, and an oversized batch cannot partially delete a
key. A successful update-plus-delete also survives reopening. Full native tests
pass (record aeaf8ac5ee654186a41931be0e221dce). This prerequisite is not yet used
by playback checkpoints; atomic library/journal integration remains outstanding.

## Checkpoint projection

The native Player can now project its progress onto the latest Core library record
without replacing current membership, metadata, notification preferences or watched
bits for other episodes. Manual completion retains the stored cleared/advanced resume
pointer for the remainder of that owner; manual unwatched permits progress while
suppressing automatic completion. Conflicting title types are rejected and future
modification times are preserved. Projection itself does not mutate Core or acknowledge
a save. Film/episode model tests, the full native suite and unsigned tvOS build pass.
The session checkpoint queue, atomic library/journal write and Swift callbacks are
still outstanding.

## Native acknowledged checkpoints

`CoreSession::checkpoint_playback` now captures one current-owner checkpoint, admits
its Core library state in memory, and writes both library buckets atomically through
the existing sequential writer. The receipt waits for that write. A concurrent save
or retirement returns busy; account transition commands also wait for the pending
checkpoint. This is a bounded explicit-save primitive, not the periodic coalescer.

The real session/HTTP test holds the writer to prove no early acknowledgement, checks
that later played progress does not change the captured receipt's stored position,
then saves the newer position. It induces a disk failure, verifies the old stored
position remains, retries and reopens encrypted storage. Retired tokens are rejected.
Full native tests and the unsigned tvOS build pass. No Swift consumer, close-save
barrier, automatic cadence or sync journal is connected yet; retirement alone still
does not imply a save. Account-mode release remains gated on journal/conflict tests.

## Native durable Close

`close_playback` now freezes progress while the final checkpoint is pending and
retires the owner only after storage success. A write failure releases the freeze
and keeps the owner available for retry. Concurrent close/save returns busy; a
retired token cannot submit progress or another close. Tests hold the actual writer
to prove no early close acknowledgement and reject late progress before releasing
the write. Disk-failure recovery and encrypted reopen remain covered. The full
native suite, unsigned device build and 15 harness tests pass. Swift Close/Home
controls and periodic/lifecycle scheduling still need this native integration.

## Playback receipt bridge

The C ABI and NativeCoreService now expose bounded Start, Seek, played progress,
Pause/Unpause, Checkpoint, durable Close and accounting-only abort requests. They
share the existing exclusive ticketed handoff slot. Receipts validate session,
ticket, playback token, action and expected progress shape; input times are unsigned
milliseconds bounded by INT64_MAX. Cancellation/timeout stops observation and does
not roll back an already admitted operation. Swift decodes prepared playback tokens
for both torrent and HTTPS sources.

The real Swift/native HTTP fixture starts a selected source, seeks without viewing
credit, reports played time, checkpoints, closes and rejects stale callbacks. All 60
Swift tests, full native tests and the unsigned tvOS build pass. An existing sign-out test's initial account
startup exceeded its one-second wait; it now waits until ready with a three-second
deadline and retains the same readiness assertion. Decoder callbacks, lifecycle
controls, periodic coalescing and resume UI remain unconnected.

## Bounded Swift accounting queue

PlaybackAccountingQueue serializes native operations with a default 32-operation
bound including the in-flight operation. Adjacent unsent played samples coalesce;
seek, pause, checkpoint and close boundaries do not. Close seals admission only
when accepted. Capacity rejection preserves prior intent and is returned to the
caller. An operation failure retains the failed operation and all pending work;
flush reports failure and explicit retry resumes the queue. No automatic replay
is claimed safe after an ambiguous timeout; caller reconciliation remains required.

Tests hold the operation boundary while submitting 1000 progress samples, then
verify the exact played/seek/save/close order. They also cover failed-close retry
and capacity rejection without eviction or falsely sealing Close. This queue is
not yet connected to the VLC callbacks or lifecycle controls. All 63 Swift tests,
the unsigned tvOS build and 15 harness tests pass.

## Decoder accounting adapter

PlaybackAccountingOwner classifies explicit decoder generations and reposition
markers before queueing Core operations. Initial/resumed decoder position, backward
movement and confirmed repositioning become Seek; ordinary played progress remains
TimeChanged. A requested reposition waits for the observed clock within 500ms of
the bounded target. Pause/seek completion checkpoint immediately, and playing
samples checkpoint at a five-second monotonic-clock cadence. Close stops admission
and drains accepted work before its durable barrier. Unknown decoder duration is 0.

Only activateDecoder may change generation; stale callbacks are rejected. Callback
groups enter the bounded queue atomically, preserving seek/pause/save as a unit on
capacity rejection. Tests cover ordered classification/cadence, stale generations
and no partial batch admission. All 65 Swift tests, the unsigned tvOS build and 15 harness tests pass. Real VLC duration/seek
markers, retained parent ownership, resume choice and lifecycle UI are not wired yet;
this is not proof of end-to-end resume or physical suspension behavior.

## VLC observations and startup coordination

VLC snapshots now carry the media's current duration and persistent reposition
markers for remote seeks and caption-refresh seeks. Requested positions are
clamped to VLC's supported integer range. The player clock exposes duration to
accessibility without changing its elapsed-time label. The owned peer journey
verifies the selected 36-second episode's duration, decoded green frame and Close.
This does not yet connect the snapshots to Core checkpoint accounting.

Repeated accounting Start calls wait for the same native receipt. Close during
Start waits for startup, then retires before a decoder can be activated. Tests hold
the native operation boundary and assert that neither the second Start nor Close
returns early. All 67 Swift tests, the unsigned tvOS build and 15 harness tests pass; strengthened assertions also pass in the
three-test accounting suite. Parent ownership and Resume/Restart UI remain required.

## Resume identity restoration

Native Player initialization now retains saved position/duration only for the
selected video's exact stored identity. A new episode starts at zero with unknown
duration; Core previously retained the preceding episode's duration. Same-video
initialization preserves progress without adding viewed time or mutating stored
library state.

The checkpoint integration test now reopens encrypted storage, creates a new Core
session, resolves metadata and a different torrent source, starts that new owner,
and inspects its paused progress. The saved film resumes at the expected 600ms with
its prior duration and viewed-time count. This proves native restoration across
source replacement; it does not prove decoder repositioning or relaunch tolerance.


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
Canonical sanitized results: [resume measurements](evidence/resume-cycles-2026-11-18.json).
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
