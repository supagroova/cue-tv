# Phase 3 — native Stremio Core bridge

Status: implementation in progress, stacked on Phase 2. This is not a completed
Core integration or a replacement for Phase 2's outstanding physical/resource
qualification. Simulator-first development remains the user's selected path.

## Pinned source and build

Core is the Git submodule at `native/stremio-core`, pinned to
`2c89236b47dfcd5e844c1dfae5737164cb616fbf`. Initialize it with:

```sh
git submodule update --init --depth 1 native/stremio-core
```

A shallow submodule avoids downloading Core's large historical object store.
`native/cue-core/Cargo.toml` uses that path; the parent Git link is the source pin.
`build-core.sh` verifies the exact revision and rejects tracked source changes.
All direct registry dependencies use exact versions and Cargo.lock pins the
transitive graph, including local-search revision
`74fefe1da2fa17d2b4ef7e3e629da00f3946ee72`. MIT notices for Core and local-search
are retained under `native/licenses`. Complete distribution notices remain part
of the release work.

```sh
cargo +1.100.0 test --locked --manifest-path native/cue-core/Cargo.toml
PLATFORM_NAME=appletvos bash scripts/native/build-core.sh
PLATFORM_NAME=appletvsimulator bash scripts/native/build-core.sh
```

Rust 1.100.0 and both Apple targets are required, as for the torrent engine. This
crate builds independently; it is not yet linked into the Swift app or exposed
through a C ABI.

## Implemented environment boundary

`NativeEnv` implements the pinned Core's actual `Env` trait with the
`env-future-send` feature. Each session has a task-local `Environment`; concurrent
and sequential effects retain that same context. Sequential effects enter a
bounded FIFO before spawning can reorder them. Queue exhaustion cancels the
context rather than silently continuing with missing effects.

HTTP uses rustls, a 15-second timeout, a 4 MiB request/response limit, HTTP status
checks, and a streamed-body bound even without Content-Length. Redirects are
currently rejected; required add-on redirect behavior must be settled before the
production browsing integration. Errors are fixed strings, without URLs, tokens
or response bodies. Core logging is not forwarded and analytics are not enabled.

Storage currently uses a per-context in-memory backend with a 16 MiB aggregate
budget. Rejected writes preserve the prior value. Cancellation serializes with
storage access and prevents later writes. This is a temporary adapter backend,
not guest relaunch persistence or a claim of secure account persistence. The
production adapter must separate credential-bearing profile data into the
platform credential store before account mode is wired.

Time is injectable per context. Construction outside an active runtime returns
an error; its regression first reproduced an unwinding Tokio panic. Cancellation
interrupts pending HTTP and scheduled effects. The Rust session now joins owned tasks on shutdown; the C ABI shutdown
acknowledgement remains part of the next bridge slice.

## Evidence

The initial seven native tests passed: real Core storage-schema migration, isolated storage and
canceled writes, JSON HTTP, Content-Length rejection, chunked-body rejection,
pending HTTP cancellation/redaction, FIFO effects with context preservation,
runtime construction failure, and injected clock/storage-budget behavior (some
tests cover more than one contract). They complete in approximately 0.45 seconds
on this Mac. The crate compiles for arm64 tvOS device and simulator; compilation
is not on-device execution evidence.

Apple CI checks out the pinned submodule, runs these tests and cross-compiles both
targets. Native/Swift-package caches are keyed by toolchain, dependency inputs and
commit. Apple CI runs on PR updates and main pushes to avoid duplicate expensive
jobs for every branch push; manual qualification remains opt-in.

## Remaining Phase 3 acceptance

- Extend the owned context/model actor beyond settings to the service operations
  needed by browsing and account integration.
- Extend the versioned C ABI and Swift service beyond settings/add-on preview;
  preserve ownership, malformed-envelope and post-stop guarantees.
- Persist guest state, version/recover storage and implement protected account
  secrets, identity changes and queued-write cancellation.
- Verify and implement the pinned upstream authentication/sign-out flow; no
  invented OAuth or device-code flow.
- Exercise action → HTTP/storage → state on the simulator and eventually device.
- Keep account/add-on data and tokens out of user diagnostics and logs.

Phases 4–7 remain required; this foundation does not narrow the original scope.


## Owned model/action slice

`CoreSession` owns the pinned Core's real `Ctx` model. Its first exposed operation
validates and dispatches `ActionCtx::UpdateSettings`, then processes Core's actual
internal messages and asynchronous storage effects. No replacement settings model
or fake Android service is used. The guest model starts from Core's default profile
and loads a stored profile only when it contains no account authentication data.

Commands use a bounded 64-entry queue. Immediate message processing has a 1,024
message budget; async effects use the environment's concurrent/FIFO scheduling.
Snapshots coalesce through a watch channel rather than accumulating events, and
expose revision, language, add-on count, pending work, stopped state and fixed error
codes. They do not serialize the full profile or raw Core events.

Stop cancels the context, joins the model owner and waits for tracked concurrent,
sequential and queued effects to be dropped. Admission and cancellation share a
lock, so work cannot be registered after the shutdown barrier. Failed model setup
also closes its environment. The shutdown regression deliberately failed when the
join was omitted, proving that cancellation alone does not satisfy its assertions.

A storage-exhaustion test reproduced stale error state poisoning the next command.
New actions now clear the prior error before processing; after freeing capacity,
a subsequent real Core action successfully persists its profile. This is not yet
a general automatic retry/sync policy (Phase 5).

Current local evidence: 13 tests pass, including real model action → state → Core
storage effect, model/storage isolation, post-stop rejection, failed startup,
effect failure recovery and shutdown with blocked/queued work. Both tvOS target
builds pass. A controlled panic regression first left the session running until
its deadline. Concurrent and sequential effect panics now cancel the environment,
and the model owner publishes a stopped snapshot with `core_task_failed`; owner
unwinds are also contained. Shutdown still joins tracked effects. This does not
catch process aborts or suppress Rust’s process-wide panic hook, so production
diagnostic handling remains part of the ABI integration.

Remaining bridge work includes the C ABI/Swift boundary, action → HTTP → model state, durable storage and account flows.

## Versioned C boundary

`native/cue-core/include/cue_core.h` defines ABI v1. Creation returns a monotonic
session ID and starts the guest model asynchronously on a two-worker runtime.
At most eight unreleased sessions exist. Commands are strict, size-limited JSON
and enter a bounded 64-entry queue; acceptance is not completion. Snapshots carry
ABI version and session identity and are copied into caller-owned buffers. A
size query does not reserve a snapshot; callers retry if the returned size grows.
Undersized buffers remain untouched. No Rust allocation crosses this boundary.

Stop requests cancellation. Status becomes complete only after the model owner
and its effects have joined; release refuses running sessions. The session stays
outside the bridge's unwind boundary so a bridge failure can still join it.
The native boundary test exercises a real settings action, invalid envelopes,
unknown fields, null-input rejection, undersized buffers,
post-stop calls, repeated stop, stale IDs and the eight-session capacity limit.
As with any C API, non-null pointers must refer to valid memory of the stated
length; this interface cannot validate arbitrary addresses.

Swift integration, production diagnostics, durable storage, account operations
and the network-driven model path remain outstanding. This C slice alone does
not satisfy the Phase 3 exit gate.

C-boundary verification: all 14 native tests pass; locked device and simulator
static-library builds pass. Swift application linkage is not yet verified.

## Swift service integration

The generated app project now builds and links both native libraries, with the
Core module map available to Swift. `NativeCoreService` serializes bridge calls
on a separate actor, validates snapshot version/session identity and bounds the
snapshot copy to 16 KiB. The model is decoded into Sendable values; raw Core
objects and authentication data are not exposed.

Concurrent stop calls await the same retirement task. The service clears its
handle only after native join and release, and checks the captured handle after
awaiting to avoid clearing a replacement session. A five-second shutdown timeout
retains the handle for retry; deinitialization makes a best-effort asynchronous
retirement. Production recovery of a permanently stuck native task remains an
explicit failure case, not permission to free live state.

The device application links successfully with both Rust archives. The focused
simulator test executes a real Core settings action through Swift, checks the
result and settled effects, overlaps two stops, rejects a post-stop command and
restarts with a fresh session ID. Removing action delivery while acknowledging
success makes the test fail (`eng` instead of `fra`), confirming it exercises the
real bridge. This is still a guest lifecycle/settings slice: catalogue/search,
add-ons, sources, account/library operations and durable storage remain required.

All 23 simulator unit tests pass after restoring delivery (2.659 seconds of test
execution). These checks do not establish physical-device behavior.


## Network-driven add-on model

The owner now includes Core's actual `AddonDetails` model and combines its effects
with `Ctx` effects. `load_addon` travels from the Swift service protocol through
strict C JSON admission to Core's load action and HTTP transport. HTTP(S) manifest
URLs (including normalized Stremio URLs) are validated before dispatch; local
file URLs, credentials, fragments and oversized inputs are rejected. Configured
URL paths/query strings stay inside Core. Snapshots expose only bounded ID/name
and loading/ready/failed state, never transport URLs or raw network errors.

A controlled HTTP test proves action → real Core manifest request → parsed model
state. A second test holds an old request open, loads a newer selection, then
releases the old request with a failure; Core ignores that stale result and keeps
the new selection. Native tests total 16 and pass. The device application build
and all 23 simulator unit tests pass; Swift also verifies local-file add-on URLs
are rejected. The successful network round trip is currently host-native evidence,
not yet simulator/device network qualification. Add-on installation, catalogue,
metadata, search, sources, same-URL retry policy and account operations remain.

## Simulator network path and retry

A test-owned Network.framework listener now serves a manifest inside the tvOS
simulator. The Swift service loads it through the C boundary and real Core HTTP
transport, then observes the expected parsed ID/name. The two focused service
tests pass in 0.133 seconds, including the network round trip; no external add-on
or companion server is required by these tests.

A host regression reproduced Core retaining a failed manifest when the user
reselected its URL. Explicit loading now unloads a failed preview before retrying;
it does not discard a successful preview or duplicate the same in-flight load.
The test first returns invalid JSON, then a valid manifest from the same URL and
observes recovery. All 17 native tests pass. Durable guest state and account/browsing operations remain required.


## Request generation isolation

A controlled A→B→A regression reproduced the first A request's late failure
changing the second A preview to failed. URL equality alone did not identify the
active request. The owner now tags asynchronous effects with the current add-on
request generation. Selection changes and failed-preview retries advance it;
reselecting the same in-flight/successful preview does not. Stale manifest results
still finish their pending-work accounting and reach the context model, but cannot
update the current add-on preview. No upstream Core source was modified.

The regression passes with the old response deliberately released before the new
response. All 18 native tests, the device app build and 24 simulator unit tests
pass. The physical Apple TV gate and remaining Phase 3 service/persistence/account
requirements are still open.

## Encrypted native persistence foundation

`Environment::persistent` can now restore and commit Core storage in an app-owned
directory. The default and the current Swift/C creation path remain memory-only.
A real Core guest test changes a setting, joins shutdown, opens a new environment
and observes the stored setting. This is native restart evidence, not yet an app
relaunch or Keychain claim.

The versioned `CUECORE1` envelope uses AES-256-GCM with a fresh random 96-bit nonce
for each write and an authenticated header. `ring` is a direct exact `0.17.14`
dependency (already present transitively); no dependency versions changed, and its
license notices are retained. Keys are supplied by the caller and are never stored
beside the ciphertext. Core JSON values and configured URLs are encrypted together.

Writes enforce a 16 MiB serialized-state limit, create a mode-0600 temporary file,
flush it and atomically rename it. Rename is the commit point; directory sync is
best-effort, so sudden-power-loss durability is not claimed. Failed pre-commit
writes preserve the prior in-memory and on-disk state. Wrong keys, authenticated
data corruption, unsupported versions and oversized files fail closed without
resetting or overwriting existing data. Filesystem work is blocking and must run
off the UI actor. Callers must exclusively own and serialize access to the
app-owned directory; cross-session/path leases are required before exposing this
through the application bridge. Missing files start empty. Corrupt files require
an explicit recovery policy rather than silent deletion.

All 22 native tests pass, including fresh ciphertext, wrong-key/tamper rejection,
quota preservation, write-failure consistency and real Core guest restart. Both
tvOS library targets compile. Remaining integration: platform Keychain keys,
exclusive storage ownership, C/Swift configuration, app relaunch/recovery tests,
identity scoping and account lifecycle. Large-state write latency and shutdown
responsiveness also need measurement before enabling this backend in the app.

CI cache restoration now falls back to the same architecture/Rust/Xcode cache
when lockfiles or the project change. Cargo/SwiftPM still resolve pinned inputs
and rebuild invalidated artifacts; this avoids discarding all downloaded sources
and unchanged compiled dependencies for a small manifest edit.

## Persistent-session ownership and C creation

The encrypted store now holds an OS file lock for its lifetime. A second open,
including an aliased path to the same directory, fails without overwriting state;
the lock is released when the store is dropped. The lock file remains in place so
future owners synchronize on the same inode. The app-owned directory must not be
renamed, replaced or modified by another component while in use.

`cue_core_create_persistent` accepts a copied absolute UTF-8 directory path and
exactly 32 key bytes. It rejects null/invalid-length inputs, parent traversal and
embedded NULs. Configuration is internal application input, not an add-on action.
The directory must already exist. Startup errors are fixed, redacted snapshot
codes. The owner explicitly drops its Core session and environment before
publishing completed shutdown, ensuring the file lock is released before a new
owner can reopen it. The C regression writes a Core setting, rejects a competing
owner, stops/releases and immediately restores the setting in a new session.

All 23 native tests pass and both tvOS library targets compile. The Swift app still
uses memory-only creation; Keychain provisioning, app-owned directory setup,
relaunch/recovery and account identity wiring remain required.

## Keychain-backed Swift guest service

`NativeCoreService.guest()` now prepares an application-support guest directory,
loads or creates a 256-bit random Keychain key, and uses persistent C creation.
The Keychain item is non-synchronizing and `AfterFirstUnlockThisDeviceOnly`; the
key is never stored in the data directory. Competing key creation re-reads the
winner rather than replacing it. Existing ciphertext with a missing key fails
explicitly and is preserved. Keychain errors expose numeric Security status only,
not key material. The default no-argument initializer remains ephemeral for
isolated diagnostics; the standalone journey must use the guest factory.

A simulator integration test uses the actual platform Keychain, persists a Core
setting, recreates the Swift/native service and verifies restoration. It then
removes only its uniquely named test Keychain item and verifies that startup
preparation refuses to generate a replacement key over existing ciphertext. This
is service-recreation evidence; full process termination/relaunch and physical
Keychain behavior remain required.

Unsigned simulator tests reproduced a Keychain-access failure. Simulator test
commands now use ad-hoc manual signing, an empty development team and a fixed
simulator-only application-identifier prefix. No developer certificate/account is
needed. Device build checks remain unsigned. All 25 simulator unit tests pass
(3.246 seconds of execution), and the device application builds. Missing/corrupt
state recovery UI, account identity/sign-out and remaining Core service operations
are still open.

All 10 short Playback Lab UI tests also pass with ad-hoc simulator signing
(100.490 seconds of test execution). Sustained playback remains manual-only.

## Installing a validated preview

The Swift/C service now supports `install_addon` with the displayed Core revision.
The model owner rejects stale revisions and non-ready previews, then dispatches
Core's actual `ActionCtx::InstallAddon` using the descriptor it fetched. Callers
cannot supply an arbitrary descriptor or substitute a transport URL at confirmation.
Snapshots include whether the ready descriptor is installed. Core still enforces
its configuration-required, locked-add-on and duplicate-install rules.

A rejected command previously stopped the entire C-owned session. A regression
reproduced that behavior with a stale install; command failures now appear as a
fixed latest-command error in the coalesced snapshot and leave the session usable.
A subsequent accepted command clears that error. Queue acceptance remains distinct
from model/effect completion; consumers must observe the resulting state.

The native fixture now previews, installs, persists and restores the add-on from
encrypted guest storage without exposing its configured URL in plaintext. The
simulator HTTP test also installs through Swift and observes the installed state
and incremented add-on count. All 23 native tests, 25 simulator unit tests and the
device application build pass. Catalogue/search/source operations, removal and
upgrade UI, account flows and full-process relaunch remain outstanding.

## Installed catalogue first-page path

The owner now includes Core's `CatalogWithFilters<MetaItemPreview>`. The snapshot
exposes bounded catalogue choices using indexes into the installed profile; a
revision check rejects stale selection. The C/Swift `load_catalog` operation
constructs its resource request from that installed descriptor, never from a
caller-supplied transport URL. Catalogue resource effects have their own request
generation so returning to a previous selection cannot admit an older resource
result into the current catalogue model.

The first view includes at most 16 catalogue choices and 24 items, with the total
catalogue count and `has_more` reported explicitly. Display names are bounded;
item identities are preserved rather than truncated, with oversized identities
excluded at this boundary. Swift's bounded snapshot buffer is now 256 KiB to fit
these views. Full enumeration, pagination, required extras/search, artwork and
empty/error/retry UX remain unfinished and must not be treated as supported by
this first-page slice.

The owned native HTTP fixture now previews and installs an add-on, checks the
catalogue resource URL Core requests and observes the parsed film. Disabling the
real catalogue model makes that test fail; restoring it passes. The simulator
fixture executes the same preview → install → catalogue chain through Swift.
All 23 native tests, 25 simulator unit tests (3.431 seconds) and the device app
build pass. Dedicated catalogue switching/pagination tests, metadata/sources,
account operations and the standalone user journey remain required.

## Catalogue-choice windows and empty results

`catalog_choices` pages through the installed catalogue list in bounded windows,
with the offset and total in the snapshot. It checks the displayed revision and
rejects invalid offsets. A 35-catalogue Core profile test reaches every window,
returns to the first, and rejects stale/out-of-range navigation. Swift exercises
the operation before loading its selected fixture catalogue.

Core's `EmptyContent` catalogue result now maps to a successful empty view rather
than a transport failure. Its regression first reproduced the incorrect failed
state. All 25 native tests, 25 simulator unit tests (3.483 seconds), and the device
app build pass. This completes choice-list enumeration; catalogue **item** windows,
remote page fetching, search/filter extras and retry UI remain required.

## CI build-size follow-up

The cached Apple run for `947ae98` passed but still took about 13½ minutes. Cache
restore consumed 66 seconds and cache save 106 seconds for a roughly 2.7 GiB
archive; this is not a proven sub-ten-minute pipeline.

CI now disables Rust debug information and incremental compilation for its native
builds only. Debug assertions and test behavior are retained, and local development
settings are unchanged. The new cache generation is keyed by toolchain/dependencies,
not every source commit, so an exact hit avoids another archive upload. Cargo still
checks source fingerprints and uses locked dependencies. The first new-generation
run is cold. Lean-profile native tests and simulator compilation pass locally;
the current Core simulator archive measures about 105 MiB versus 269 MiB with the
normal debug settings. End-to-end CI timing remains to be measured.

## Catalogue content windows and remote pages

`catalog_items` selects a revision-checked offset in the loaded content. It serves
24-item windows from Core's retained pages and invokes Core's `LoadNextPage` action
when the requested offset reaches the loaded end and Core advertises another page.
The snapshot reports its content offset, loading/error state and whether more
results exist. Earlier loaded windows remain available. Catalogue reloads always
advance the resource generation, including repeated loads of the same catalogue.

A native HTTP regression serves 27 items, then three, then an empty response. It
checks the actual `skip=27`/`skip=30` URLs, exact item order across windows, end-of-
results behavior, backward navigation and rejection of an unavailable offset. The
simulator fixture now runs through those remote pages via Swift as well. All 26
native tests, 25 simulator unit tests (3.337 seconds) and the device app build pass.
One device build attempt was killed while running XcodeGen, before compilation;
the tool then responded normally and the recorded retry succeeded.

Views and HTTP responses remain bounded, but Core currently retains loaded pages
until a catalogue reload/selection change. Long-browse memory behavior, page-error
retry, overlapping response coverage, search/filter extras, artwork and the
standalone browsing UI remain required before this is merge-ready.

### Overlapping catalogue pages

Presentation windows now deduplicate by `(type, id)` across loaded pages, preserving the first occurrence. Core retains the raw page contents so its remote `skip` remains the number of server results, not the number of displayed unique titles. The network fixture returns two overlapping titles on its second page and asserts that the final request uses `skip=32` while the display has 30 unique titles.

The strengthened test failed with repeated film-25 and film-26 before the change; all 26 native Core tests passed after it (`task.py verify phase-3-core`, evidence `5fdb0f031a98415b8281b28a4ffc9888.log`). This does not resolve the retained catalogue cache bound or page-error retry requirements.
