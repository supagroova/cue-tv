# Cue TV — staged implementation plan

17 November 2026 · Draft execution plan · Based on [PRD](PRD.md) and [research](RESEARCH.md)

## Objective and ordering

Prove raw torrent playback on the user's **Apple TV 4K A2737, 64 GB, user-reported tvOS 27.2**, before investing in account integration or catalogue UI. Normal playback must run entirely within Cue TV, without Android, DietPi, the development Mac, a paid debrid service or an external player.

This document plans implementation; none of the new engine, player or physical-device tests described here has been completed. The existing unsigned build and five tests establish only the old application baseline. The PRD's proposed defaults remain planning assumptions: one profile, public BitTorrent v1, films and series, 1080p SDR initially, guest/account modes and no unattended seeding.

The first milestone deliberately moves the torrent/player proof ahead of Stremio Core integration. A fixed permitted test torrent isolates the highest risk without waiting for authentication and add-ons. This changes the ordering of PRD G1/G2, not their acceptance requirements. A successful test screen is an intermediate milestone, not the finished Stremio product.

| Stage | Result | Depends on | Release gate |
|---|---|---|---|
| 0 | Reproducible signed app on the real device | Device access and signing | G0 |
| 1 | Direct torrent plays, seeks and stops on Apple TV | 0 | Early torrent portion of G1 and G2 |
| 2 | Engine, player and cache architecture selected | 1 | Complete playback feasibility |
| 3 | Native Stremio Core and local state bridge | 2 | Remaining G1 |
| 4 | Stremio browsing, sources and episode selection | 3, playback contracts from 2 | G3 discovery |
| 5 | Resume, library, account sync and recovery | 4 | G3 complete |
| 6 | Reliability, compatibility and independence qualification | 5 | G4 technical |
| 7 | Personal release and reproducible handover | 6 | G4 delivery |

Do not commit a calendar before stages 0–2 resolve toolchain and engine feasibility. At each gate record actual effort, remaining risks and revised estimates. Hardware interaction and account sign-in are user-assisted steps; repository publication is a separate decision.

## Working conventions and evidence

Preserve the current extracted source and its manifest. Start implementation in an isolated local checkout/branch when repository access permits; otherwise use a separately named working copy and retain provenance. Do not overwrite the existing build artifacts. No remote branch or PR is required to execute this plan.

Use the existing XcodeGen `apple-tv/project.yml` as the project source of truth. Apply personal bundle ID and team overrides through ignored signing configuration, including unique URL schemes and Keychain service names. Never commit credentials, provisioning profiles or add-on tokens. The existing `TVService.swift` is a concrete Android HTTP client and `WatchModel.swift` owns it directly: introduce a protocol and injection boundary instead of pretending a native service is a drop-in subclass.

For every gate produce a small evidence bundle: app/source revision, dependency pins, toolchain versions, device model/OS build, fixture identity and permission, procedure, expected/observed result, timings and redacted logs. Label results **pass**, **fail** or **not run**. Simulator tests do not count as physical playback evidence. Keep signed binaries/profile details separate from shareable reports.

## Stage 0 — installable device baseline

**Tasks**

1. Inspect installed Xcode, SDKs and command-line selection; pair the Apple TV and record its actual OS/build, model and available storage. Verify support for the reported tvOS 27.2 rather than infer it from the older simulator build. If incompatible, identify the required supported Xcode before changing toolchains; do not downgrade the device.
2. Verify available signing identity/team and provisioning entitlement. Configure a separate personal app identity, display name and URL scheme. Record signing expiry/renewal from the actual profile.
3. Regenerate the project, compile the unchanged baseline, install it, launch it from the Apple TV Home screen and collect a device log. Preserve a baseline revision and reproducible build command.
4. Add a debug-only “Playback Lab” entry that does not require Android pairing. It shows fixture selection, Start/Cancel, current phase, error and sanitized measurements. Keep it behind a build flag and exclude it from the eventual release UI.
5. Create a fixture manifest: small H.264/AAC MP4, an MP4 with end-of-file metadata, a multi-file torrent with two distinct episodes and a sample file, and later an MKV/HEVC/subtitle set. Record checksums, sizes, codecs, durations, torrent hash, file indexes and usage permission. Prefer owned/generated footage for deterministic tests.

**Exit:** signed app launches on A2737 with debugger detached; Playback Lab opens with no server; baseline and fixtures are documented. Signing/toolchain failure blocks device claims but does not prevent host-side fixture preparation.

**Deliverables:** device baseline report, signing setup instructions, fixture manifest and baseline commit. **Rollback:** remove only the new test app; leave the brother's app and Pi untouched.

## Stage 1 — prove direct torrents first

### 1A. Embed the engine

1. Pin librqbit and its dependency lockfile; use the research revision only as a starting candidate. Build an ARM64 tvOS static library and simulator slice with an explicit Rust version, target, SDK and enabled feature set. Inventory incompatible dependencies and compile only required capabilities.
2. Export a narrow C ABI with opaque engine/session handles, versioned request/event payloads, explicit buffer free functions and defined shutdown. Do not expose Rust types, unwind across the ABI or call Swift UI from worker threads. Keep blocking work off the main actor.
3. Add Swift engine wrapper operations for create session, acquire metadata, enumerate files, select file, read/seek bytes, snapshot statistics and cancel. Include a generation/session ID in every callback; drop results from obsolete sessions.
4. Verify piece hashes before serving bytes. Validate torrent sizes, file indexes and normalized paths before storage creation; constrain all writes to a per-session cache directory.

**Evidence:** library loads on the real device, retrieves metadata and reads verified bytes from a permitted torrent. A build alone is insufficient.

### 1B. Establish which networking works

Run isolated cases for HTTP tracker, HTTPS tracker, UDP tracker and DHT. Disable alternative discovery paths in each case and record effective configuration so one working mechanism cannot mask another's failure. Use a known direct peer only as a diagnostic control, never as the discovery acceptance result. Test cold metadata acquisition separately from a bundled `.torrent` file.

A controlled seeder is a test fixture and may initially run on the Mac. Record Wi-Fi conditions and seeder availability. Later repeat a permitted public-swarm case with the development Mac disconnected and Pi off. Lack of a healthy public swarm is an inconclusive test, not proof of engine failure.

Diagnose failures by layer: platform build, socket creation, DNS, tracker exchange, peer handshake, metadata exchange, piece transfer. Inspect relevant Apple networking permissions and runtime errors before changing entitlements or transport exceptions. Do not assume another project's UDP failure applies to this app.

### 1C. Connect verified bytes to an in-app player

1. Start with AVPlayer and the constrained MP4 fixture to minimize integration work. Run a bundled local-file control first so decoder failure can be separated from torrent transport.
2. Implement a loopback-only HTTP byte-range adapter, or choose custom player I/O if testing shows that simpler. The first approach must support HEAD and normal GET, single byte ranges including suffix/open-ended ranges, correct 200/206/416 responses, exact content lengths, EOF and cancelled clients. Explicitly handle unsupported multi-range requests. Avoid loading whole files into memory.
3. Use an unpredictable active-session URL, reject other paths/methods and close the listener on session termination. Do not expose the listener to the LAN. Keep any transport-security exception narrow.
4. Translate reads into piece priorities, including header and tail/index reads. Cancel obsolete blocked reads on seeks/source changes. Use a bounded read-ahead window rather than relying on sequential download alone.
5. Show metadata → discovery → buffering → playing transitions. Instrument first-frame time, verified bytes available at first frame, total file size, peer count, transfer rate, cache use and player errors.
6. Exercise pause, seek outside downloaded regions, Back, start again and app backgrounding. Ensure player, byte adapter, engine and pending work terminate together.

### 1D. Demonstrate the milestone

Run a cold-cache MP4 session on A2737. First frame must occur before the selected file is fully downloaded. Play for at least 10 minutes, seek to two verified uncached positions, pause/resume and stop. Repeat source start/stop five times. The UI must remain cancellable when no peers are present. Verify transfer and pending read work cease after stopping; record a bounded cleanup deadline and measurements.

Then run a separate independence demonstration with a permitted external swarm: Pi off, development Mac disconnected after installation, no Android service, no debrid credentials and no external-player handoff. Save redacted on-device measurements for later retrieval. This proves playback independence; full Stremio journey independence comes in stage 6.

**Exit:** reproducible first-frame-before-completion evidence, uncached seeks, cleanup and independence results. The PRD's final 9/10 performance targets are qualified in stage 6, not inferred from this short smoke test.

**Decision branches:**

- If librqbit cannot build or transfer on tvOS, record the failing dependency/protocol and a bounded repair experiment. If unresolved, run the same fixtures through libtorrent with a C/C++ bridge. Keep Swift contracts unchanged.
- If local MP4 playback works but streamed playback fails, inspect range handling and piece availability before replacing the player. If the adapter/player combination remains unsuitable, evaluate VLCKit or MPVKit with the same fixture.
- If no engine produces verified on-device torrent playback, stop product integration and report the blocker. Debrid or Pi playback cannot satisfy this milestone.

**Deliverables:** Playback Lab build, engine build recipe, range/seek contract tests, device evidence and first architecture decision record. **Rollback:** remove the lab implementation or revert its commit; preserve failure evidence and baseline.

## Stage 2 — choose production playback and bound resources

1. Compare VLCKit and MPVKit using the same MP4/MKV, H.264/HEVC SDR, AAC, alternate-audio and SRT/WebVTT fixtures. Record exact profiles and hardware/software decoding behavior. Choose one production player based on actual compatibility, maintainability, reproducible builds and dependency notices. AVPlayer may remain only if the agreed matrix can be met; do not quietly drop MKV.
2. Implement a `PlaybackSessionCoordinator` with explicit states: idle, resolving, discovering, buffering, playing, paused, seeking, stopping, failed. Serialize state changes; every new Play first cancels the old generation. Publish a stable UI snapshot, not engine internals.
3. Define a cache policy using measured free space, a reserve and an app cap. Account for allocated blocks, metadata and unselected boundary pieces, not just selected-file length. Enforce limits during writes, not only at preflight. Keep transient media separate from account state.
4. Test files larger than the cache cap. Choose either proven rolling piece eviction/re-download with correct availability bookkeeping, or explicit preflight rejection of oversized sources. Simply deleting engine-owned pieces is not a safe eviction implementation. Document the supported size behavior in UI and the PRD.
5. Measure memory/CPU/disk during sustained playback and repeated seeks. Set numeric budgets from evidence and record them before release qualification. Decide pause/upload behavior; stop transfer on playback exit/background and make the policy visible.
6. Implement failure categories and bounded timeouts for invalid metadata, discovery, buffering, read failure, decoder failure, unsupported format, low storage and cancellation. No infinite spinner or automatic paid-source fallback.

**Exit:** selected engine/player/version/feature set, supported media matrix, numeric resource budgets, proven cache enforcement and source-switch cleanup. Test tracker/DHT failures explicitly; unresolved required protocols block completion or require a visible scope decision.

**Deliverables:** architecture decisions, dependency notices/build manifest, resource policy and playback test report. **Rollback:** retain stage 1 fixture path; do not expand production UI until these choices hold.

## Stage 3 — embed Stremio Core

1. Pin Core and review its actual action/state/Env interfaces. Implement a separate Rust application-core bridge; reuse toolchain infrastructure but keep its lifetime independent from torrent sessions.
2. Implement HTTP, time, task execution and storage adapters with cancellation, response-size limits and deterministic test doubles. Define C ownership, thread affinity, event ordering and teardown exactly as for the engine bridge.
3. Wrap Core in a Swift service protocol for catalogue/search/metadata, installed add-ons, sources, account and library operations. Map state into native view models. Do not emulate Android's entire API or fabricate streaming-server health responses.
4. Add guest startup plus supported Stremio account connection/sign-out. Verify the current upstream authentication flow during implementation; do not invent an OAuth/device-code flow. Use system entry or supported browser assistance where necessary.
5. Protect tokens and configured URLs; scope state by identity, version serialized state and test corrupt/missing storage. Sign-out cancels sessions and clears account-specific data and queued writes.

**Exit:** on-device Core action → network/storage effect → state update; guest relaunch; account success/failure/sign-out; no secrets in logs. Fixtures cover bridge ownership, malformed events and callbacks after shutdown.

**Deliverables:** Core bridge and service contracts, persistence schema/migration policy, account-flow tests. **Rollback:** revert the service wiring while preserving Playback Lab; no dependency on a logged-in account for torrent diagnostics.

## Stage 4 — build the standalone Stremio journey

1. Inject the native service into `WatchModel`; replace pairing-first navigation with guest/account setup. Remove Android address/event-stream assumptions from standalone mode and exclude YouTube/remote-control flows.
2. Load supported manifests and catalogue, search, metadata, stream and subtitle resources. Bound parallel requests, cancel obsolete searches and isolate add-on failures. Keep unknown size/seed data explicitly unknown.
3. Implement film details and explicit season/episode selection with stable video IDs. Preserve focus when moving between lists, details, sources and player.
4. Normalize sources into typed torrent or compatible HTTPS descriptors. Validate info hash, magnet parameters, file index and URLs; separate tracker and web-seed hints. Raw-torrent acceptance fixtures must disable web seeds so HTTP media cannot mask peer transfer.
5. Map source file indexes against engine metadata. If the index is absent, match episode identity conservatively or offer file selection. Never assume the largest file is the episode. Test a sample-plus-two-episode torrent and ambiguous names.
6. Connect user-selected sources to the coordinator. Do not start transfers on focus. Route unsupported/private/v2-only source cases to clear messages; do not add public discovery to private torrents.
7. Add in-player audio/subtitle controls and available add-on subtitles with URL/size validation, cancellation and temporary-file cleanup.

**Exit:** browse/search → correct episode → chosen raw torrent → in-app playback with no Android configuration. Fixture contracts and one permitted live add-on exercise the same path.

**Deliverables:** standalone UI, source adapter, episode/file-selection tests and recorded device journey. **Rollback:** keep the previous signed test build; isolate UI regressions from engine diagnostics.

## Stage 5 — resume, library and recovery

1. Save title/video identity, position, duration and timestamps periodically and on pause/exit. Keep source URL separate so expiring sources can be resolved again. Select a save cadence consistent with the PRD's 15-second resume tolerance.
2. Implement library add/remove and watched changes through Core. Define completion threshold, manual override precedence, conflict rules and account switching in an explicit decision record; inspect upstream semantics before implementation.
3. Persist a bounded, identity-scoped sync queue if Core does not already supply equivalent behavior. Use idempotent operations, capped retry/backoff and visible recoverable errors. Never replay one account's writes into another.
4. Handle Home/sleep/interruption/network changes: save promptly, cancel transfer and blocked reads, restore selected title and offer resume on return. Do not depend on a background timer firing after suspension.
5. Add diagnostics with error category, engine phase and local counters. Redact account tokens, tracker passkeys, configured URLs and viewing identifiers by default; no automatic upload.

**Exit:** ten exit/relaunch cycles meet resume tolerance; offline edits reconcile under documented rules; cache removal leaves account state intact; sign-out and identity changes leak neither data nor callbacks.

**Deliverables:** state/sync specification, lifecycle tests and recovery UX. **Rollback:** disable sync behind a development flag while retaining local playback; do not ship account mode until its data behavior passes.

## Stage 6 — qualify the full app on A2737

Run the PRD matrix using the final chosen dependencies and frozen fixtures:

- Ten controlled cold-cache starts: at least nine first frames within 20 seconds.
- Ten uncached seeks: at least nine resume within 10 seconds.
- Three 60-minute baseline-media sessions without crash, wrong file or sustained A/V drift.
- Twenty source switches without orphaned transfers or unbounded memory/cache growth; pause/Back/cancel respond within one second even with stalled discovery.
- Zero peers, unavailable tracker, network loss/recovery, invalid metadata, invalid paths/indexes, unsupported codec, simulated write exhaustion, low-space and cache-purge recovery. Inject quota failures safely instead of filling the user's device.
- Ten resume cycles within the 15-second tolerance, including suspension and relaunch; account isolation and failed-sync recovery.
- Full guest and connected Stremio journeys with Pi off and development Mac disconnected, including catalogue, source selection and raw peer playback. Record public-swarm timings separately from controlled thresholds.
- Siri Remote focus/accessibility review; upgraded build preserves state; release configuration has no lab controls, broad debug transport exceptions or external-player dependencies.

Record numerical memory/cache ceilings from stage 2 beside each sustained test. A retry does not erase a failed run; capture the fix and rerun the affected case. Use simulator/host automation for parsers, byte ranges, state transitions and persistence, physical tests for network/decoder/lifecycle behavior.

**Exit:** all mandatory cases pass or an explicit PRD amendment records a supported limitation. Do not relabel a failed direct-torrent test as an HTTPS success.

**Deliverables:** release qualification matrix and known limitations. **Rollback:** revert the regression and repeat affected tests; retain the last passing build with its evidence.

## Stage 7 — personal release

Produce a reproducible source/dependency manifest, exact build commands, signing and device-install instructions, personal upgrade/renewal/removal guide and troubleshooting notes. Verify install and same-identity upgrade on the user's Apple TV with debugger detached. Explain cache limits and active-session upload behavior in settings/help. Inventory notices for the actual combined binary and retain required source/build materials.

Keep team-specific configuration and credentials local. Offer a clean branch/patch for the brother's repository only when the user chooses that destination; publishing a PR is not implied by this plan. Preserve the previous known-good signed build where its profile remains valid, and document schema compatibility before promising downgrade support.

**Exit:** user can launch, watch, resume and recover without developer assistance or a companion server. Handover clearly distinguishes supported formats from untested 4K/HDR/audio capabilities.

## Requirement coverage

| PRD requirements | Implementation stages | Main evidence |
|---|---|---|
| FR-01 standalone | 1D, 4, 6 | Playback and complete-journey independence |
| FR-02 Core/account | 3, 5 | Bridge, auth, sign-out and isolation |
| FR-03 add-ons | 4 | Resource contracts and live fixture |
| FR-04 episode choice | 4 | Multi-file episode selection |
| FR-05 torrent inputs | 1, 4 | Validated descriptors and raw peer transfer |
| FR-06 peer transfer | 1B, 2, 6 | Isolated discovery protocols and verified pieces |
| FR-07 controls/media | 1C, 2, 4, 6 | Player/format and remote-control matrix |
| FR-08 one session | 1, 2, 6 | Cancellation and 20 switch cycles |
| FR-09 progress | 5, 6 | Ten resume cycles and sync retries |
| FR-10 library | 3, 5 | Persistence and account reconciliation |
| FR-11 cache | 2, 6 | Quota, purge and low-space tests |
| FR-12 lifecycle | 1, 5, 6 | Home/sleep/interruption and cleanup |
| FR-13 diagnostics | 1, 5, 6 | Error differentiation and redaction |
| FR-14 installation | 0, 7 | Signed launch and upgrade |
| FR-15 focus/accessibility | 4, 6 | Complete Siri Remote journey |

## Immediate execution backlog

Start with stage 0, then 1A–1D in order. The first implementation session should finish by recording the toolchain/device/signing baseline and producing either a signed Playback Lab launch or a precise installation blocker. Next, build and run the embedded engine before integrating Core or redesigning catalogue screens.

The decisions still needed during execution are signing/team capability, exact fixture/media profiles, measured cache/memory limits, engine/player selection and eventual repository destination. They do not require another product questionnaire before preparing the first proof.
