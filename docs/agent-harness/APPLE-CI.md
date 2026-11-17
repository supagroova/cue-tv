# Apple TV CI execution

The workflow keeps native host tests, unsigned tvOS device compilation, simulator unit tests and all short Playback Lab UI tests required. The ten-minute playback qualification remains an explicit manual-dispatch option.

## Independent jobs

- **Native tests and tvOS device build** runs the Core and torrent host suites, then `make apple-build`. The app's pre-build scripts compile both native libraries for the device target.
- **Apple TV simulator tests** starts simulator boot after cache/tool setup, builds the fixture seeder, and runs `make apple-ci-test`. One Xcode invocation builds both native libraries for the simulator and runs the unit target plus `PlaybackLabTests`.
- **build-and-test** preserves the existing aggregate check name. It passes only when both jobs succeed; a failed, skipped or cancelled job cannot produce a green aggregate.

The jobs use separate runners, output directories, artifact names and cache keys. Both may restore the previous combined cache once, but discard the unused tvOS architecture before saving their own cache. The first run can still spend time creating those caches. External Action revisions, XcodeGen archive/checksum and Rust versions stay pinned. Cargo resolution stays locked.

## Local combined check

From the repository root, with the pinned toolchain, XcodeGen and an available tvOS simulator, start the owned fixture seeder using the existing Playback Lab procedure, then run:

```sh
make apple-ci-test DESTINATION='platform=tvOS Simulator,id=YOUR_SIMULATOR_UDID'
```

This selects `SittingRoomTVTests` and `SittingRoomTVUITests/PlaybackLabTests` in one `xcodebuild test` invocation. It does not select backend-dependent UI tests or the manual long-playback class. Separate `apple-test`, `apple-lab-test`, `apple-ui-test` and `apple-long-test` commands remain available.

## Timing evidence and limits — 2026-11-17

[The prior serial run](https://github.com/supagroova/cue-tv/actions/runs/44105292891) took **10m49s**, including:

| Work | Duration |
| --- | ---: |
| Core tests and explicit cross-compiles | 61s |
| Torrent host tests and seeder build | 42s |
| Device app build | 107s |
| Simulator boot wait | 43s |
| Unit-test build/run | 163s |
| Short UI-test build/run | 149s |
| Setup, cache transfer, artifacts and cleanup | 84s |

Running independent device/native work alongside simulator work should bring the job critical path to roughly **6–7 minutes once runners are allocated**, with some extra runner/setup usage. This assumes two macOS runners are available; total push-to-result elapsed time also includes queue delays. This is an estimate from the measured steps, not a proven remote speedup. Cold caches, runner queueing and simulator startup can change it. Compare the first and warm subsequent split-job runs, reporting both job duration and total workflow elapsed time, before declaring the timing issue resolved.

Local validation on branch `codex/apple-ci-speed`, based on `614b74d`:

- The combined real simulator run passed all **25 unit tests and 10 short UI tests**, using the existing owned peer fixture. UI test execution was 100.641s. The isolated worktree required native recompilation, so its total runtime is not a comparable CI benchmark.
- Shell-wrapper tests first failed for the missing combined action, then passed. They verify both required selectors in a single Xcode invocation and propagation of Xcode failure.
- All 12 portable harness tests pass. Workflow/composite YAML parses, `bash -n` passes, and pinned `actionlint` 1.7.12 passes (ShellCheck integration disabled because it was unavailable locally).
- Local task evidence: `apple-ci-speed`, combined run log `1571562255ff47d384cee3006a5af131.log`; actionlint log `885bc261e7204d438fd9824f53321104.log`. An initial isolated-worktree run failed because a shallow Git repository cannot serve as a submodule reference; a normal pinned submodule clone fixed that setup issue before the successful run.

No product assertions were removed or relaxed. These checks do not replace physical Apple TV qualification. Remote verification is recorded below.


## Remote measurements and deferred-boot experiment

| Run | Native/device | Simulator | Workflow elapsed including queues |
| --- | ---: | ---: | ---: |
| [First split run](https://github.com/supagroova/cue-tv/actions/runs/44109834691) | 4m43s | 11m57s | 17m45s |
| [Warm role-cache run](https://github.com/supagroova/cue-tv/actions/runs/44113416860) | 4m13s | 8m52s | 9m02s |

Both runs passed every required lane. The first restored the legacy 1.85 GB cache and created separate role caches. The second hit those exact role caches (approximately 1.59 GB simulator and 1.61 GB device), skipping cache saves. Its simulator run passed 28 unit tests and 10 short UI tests; the test-count increase came from product work. Both runners started within four seconds of workflow creation on the warm run, compared with queue delays of 5m40s and 10m39s on the first run.

The original 6–7 minute estimate was not achieved. Simulator setup took 186s then 115s, compared with 55s then 50s on the native lane. In the warm run, the simulator cache-hit message appeared about 47s after setup began; cache download/extraction then took about 45s, versus 34s on the native lane. This repeated slowdown supports testing whether simulator boot competes with setup for CPU and disk resources, but does not prove causation.

The next experiment moves only simulator boot after cache/tool setup, retaining overlap with fixture seeder compilation and the existing readiness gate. All required tests, pinning and optional sustained playback remain intact. Workflow lint and portable checks validate the change locally; another remote run is needed to measure its effect. No speedup is claimed for this ordering change yet.


## Exact native artifact reuse

Fresh checkout timestamps repeatedly rebuilt local Rust crates despite dependency-cache hits: recent simulator logs show roughly 35–53s for Core, 20–30s for the torrent library, and 36–103s for the fixture seeder. The simulator lane now has a separate exact-content cache for its two static libraries and seeder executable. Its fingerprint includes every tracked native source, recursive submodule contents and Gitlink revision, native build scripts, project configuration, workflow/setup configuration, and explicit Rust/Xcode/platform/profile identity. There are no partial restore keys.

An exact hit validates every artifact against a saved SHA-256 manifest before copying any into build outputs. Missing, empty, corrupt or source-mismatched artifacts fail closed. Only CI simulator builds with the matching fingerprint bypass Cargo compilation; device builds and all native/Swift/UI tests still execute. Cache misses compile normally and stage artifacts only after successful simulator tests. Long playback remains manual-only. The cache action remains pinned to its existing full SHA.

Portable tests cover exact restoration, a cache miss, source invalidation and corruption without partial installation. Remote cold-population and subsequent exact-hit runs are still required before claiming a measured latency reduction; source changes correctly miss this cache.
