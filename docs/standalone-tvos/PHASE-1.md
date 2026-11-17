# Phase 1 — embedded torrent transport

Status: controlled-peer simulator playback verified; phase acceptance still in progress.

The tvOS app links an embedded librqbit Rust static library. A debug lab action starts a fresh torrent session in the app sandbox, selects file index 0 and plays an unguessable loopback HTTP endpoint through AVPlayer. Native work runs on a two-thread Tokio runtime. C handles are IDs, snapshot strings have explicit allocation/free ownership, and stopping invalidates the handle and cancels transfer. The Swift owner rejects stale polling results after stop/restart. Dismissing or backgrounding the lab stops player and transport.

The current fixture peer is a separate local seeder on the development Mac, solely to supply generated test content. The app performs the download and stream adaptation itself. This does not introduce a production backend requirement. Public discovery and magnets are not implemented in the bridge yet; the proof deliberately uses known peer coordinates.

## Reproduce

Install Rust 1.100.0 and the `aarch64-apple-tvos` and `aarch64-apple-tvos-sim` targets. Xcode and XcodeGen are also required. Native dependencies use a pinned git revision and Cargo.lock. See `native/cue-torrent/NOTICE.md` for the narrow tvOS socket patch.

Run `cargo +1.100.0 test --locked --manifest-path native/cue-torrent/Cargo.toml`. In another terminal run `cargo +1.100.0 run --locked --manifest-path native/cue-torrent/Cargo.toml --example seed`, then `make apple-lab-test DESTINATION='platform=tvOS Simulator,id=YOUR_SIMULATOR_ID'`. CI starts and stops its own fixture seeder.

## Evidence

- Native byte-range boundary and two C ABI invalid-input/idempotent-stop tests pass.
- Real peer integration test passes: empty download directory, separate seeder, suffix request first, HEAD, invalid range (416), full file byte comparison, endpoint unavailable after stop. Deadline is 45 seconds.
- Seven existing/fixture Swift unit tests pass with the Rust library linked.
- Two tvOS simulator UI tests pass: local decoder start/stop and direct torrent fixture playback clock advancing beyond 0.5 seconds. Simulator runtime is tvOS 26.5.
- Full app, including the final Rust bridge, builds unsigned for the Apple TV device target.

These short deterministic tests do not establish sustained playback, discovery, uncached long seeks, bounded cache, restart stress or hardware qualification. Remaining Phase 1 acceptance: magnet metadata, public tracker/DHT peer discovery, multi-file selection, larger seek fixture and cancellation/error stress. Later phases must not treat this proof as completion of those gates.
