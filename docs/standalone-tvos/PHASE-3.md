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
interrupts pending HTTP and scheduled effects. Joining all owned tasks and the
FFI shutdown acknowledgement remain part of the next bridge slice.

## Evidence

Seven native tests pass: real Core storage-schema migration, isolated storage and
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

- Drive real Core models/actions and asynchronous state effects through an owned
  actor, with bounded events and ordered shutdown.
- Add the versioned C ABI and Swift service; test malformed envelopes, ownership,
  stale callbacks and post-stop calls.
- Persist guest state, version/recover storage and implement protected account
  secrets, identity changes and queued-write cancellation.
- Verify and implement the pinned upstream authentication/sign-out flow; no
  invented OAuth or device-code flow.
- Exercise action → HTTP/storage → state on the simulator and eventually device.
- Keep account/add-on data and tokens out of user diagnostics and logs.

Phases 4–7 remain required; this foundation does not narrow the original scope.
