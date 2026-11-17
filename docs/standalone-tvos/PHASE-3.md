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
