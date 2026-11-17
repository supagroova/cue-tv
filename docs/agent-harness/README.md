# Cue TV agent harness

The harness adapts customer-insights engineering practices to a native, personally sideloaded tvOS app. Start at [AGENTS.md](../../AGENTS.md). The current [implementation plan](../standalone-tvos/IMPLEMENTATION-PLAN.md) is the roadmap; detailed task plans use [the task template](templates/task.md). Existing app behavior has not changed.

## Commands

Run from the repository root with Python 3.9+ and Make:

```sh
make doctor
make check
make apple-build
make apple-test DESTINATION='platform=tvOS Simulator,id=YOUR_SIMULATOR_UDID'
```

`doctor` inventories tools without installation or pairing. `check` runs portable harness integrity checks and their tests; it does not compile the app. `apple-build` generates the Xcode project and builds unsigned for tvOS. `apple-test` requires an explicit simulator destination and runs the existing unit-test target. Use `make apple-ui-test` for UI tests against the configured backend/fixtures; they may not support standalone flows yet. No command installs on the Apple TV. Logs and derived data live under ignored `.agent-work/`.

The native Rust workspace is not present yet, so no cargo command is advertised as passing. When introduced, add pinned toolchain/dependency manifests, host tests, formatting/lint checks and tvOS compilation to the project gate in the same change.

## Local source and Git

The working folder at `~/Development/tvOS/cue-tv` is now a full Git checkout on `codex/tvos-agent-harness`, based on `codex/standalone-tvos-plan`. The earlier extracted snapshot is preserved alongside it as `cue-tv-snapshot-backup`. GitHub CLI authentication works; SSH key access did not, so cloning used HTTPS with `gh auth git-credential` as a command-local helper. No global Git credential configuration was changed. The documentation backup branch remains intact.

If using another extracted snapshot, `doctor` identifies it and Git-only commands fail clearly. Never initialize an unrelated root history or upload deletions for upstream files missing from a snapshot.

Optional local hook setup in a real checkout:

```sh
make hooks
```

It refuses to overwrite a different configured hooks path. The hook runs the staged sensitive-path check only. CI and explicit checks remain necessary because hooks can be bypassed and connectors do not run them. No global Git settings, provider permissions, installed plugins or account review settings are changed.

## Task evidence and handoff

```sh
python3 scripts/agent/task.py start stage-1a
python3 scripts/agent/task.py verify stage-1a -- make check
```

A start creates an immutable task identity and records Git HEAD or snapshot status plus a content digest. Each verify writes a unique log and JSON result, including exit status and before/after digests. A failed command is recorded as failed. Evidence contains the actual command arguments: never pass secrets on the command line. Repeated runs retain prior outcomes. The tool never marks the product task complete; review against acceptance criteria and any physical evidence first. Use [HANDOFF.md](HANDOFF.md) for durable summaries; keep raw logs local unless sanitized for sharing.

## Review and CI

Harness CI checks portable logic with read-only repository permission, no secrets and no PR-target execution. It runs only for harness-related changes and does not invoke upstream Android deployment. Existing upstream CI is left unchanged. The Apple TV workflow runs on every push and pull request, and supports manual dispatch. It uses macOS 26 with Xcode 26.6, installs XcodeGen, builds unsigned through `make apple-build`, selects and boots an available tvOS simulator, then runs the existing unit tests through `make apple-test`. Missing simulator/runtime or test failures fail the job. Logs and xcresult bundles are retained as CI artifacts for seven days. UI tests needing a backend and physical-device playback are separate gates. Adding a required branch-protection check or paid/scheduled AI reviewer is a separate repository setting change.

Reviews assess correctness/security, ownership/maintainability, verification and rollback. Physical tests include real torrent transfer, no web-seed/debrid substitution, first frame before full download, uncached seek and cleanup. [Device evidence template](templates/device-proof.md) separates proposed thresholds from measured results.

Codex/Claude skills share files; their actual runtime discovery should be checked in a fresh session opened at this repository. No local command interception is assumed. A read-only reviewer is a role instruction, not an OS sandbox. Provider switching uses the handoff and current Git state, not guessed transcript paths.

See [PORTING.md](PORTING.md) for every source capability's disposition and provenance.
