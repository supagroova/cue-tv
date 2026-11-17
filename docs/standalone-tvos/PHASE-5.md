# Phase 5 — resume, library and recovery

Status: title-detail library membership works through Swift/FFI and Core, with
encrypted-reopen and cancellation tests. Paged library browsing is connected to
the UI; film and episode watched controls now use acknowledged Core saves.
Playback checkpoints, sync and broader lifecycle work remain incomplete.
Earlier phases retain their outstanding gates.

The contract and ordered implementation gates are in
[decision 003](decisions/003-resume-library-sync.md). Source inspection found a
180-second Core progress-write throttle and whole-second sync comparisons;
relying on those unchanged would leave resume and conflict requirements unproven.

Branch: `codex/tvos-phase-5`, stacked on `codex/tvos-phase-4` at `0bbecfa`.
Next: bridge Core Player progress and acknowledged checkpoints, and cover UI failed-save
retry/disappearance and multi-page library navigation. Device
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
