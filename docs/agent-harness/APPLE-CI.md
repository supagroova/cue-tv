# Apple TV CI on main and feature branches

This standalone CI change adds Apple TV verification to the existing main baseline without bringing in the standalone player, Rust libraries, or the implementation PR stack.

## Execution

- Device and simulator work run on independent macOS runners, with a final `build-and-test` check requiring both to succeed.
- Rust host tests run for each native crate present in the branch. Xcode's project build phases own device/simulator native compilation; the workflow does not cross-compile Core separately and then rebuild it in Xcode.
- Simulator boot starts after cache/tool setup and overlaps fixture seeder compilation where present. A readiness gate still runs before tests.
- `make apple-ci-test DESTINATION='platform=tvOS Simulator,id=YOUR_UDID'` generates the project once and invokes Xcode once for all unit tests plus `PlaybackLabTests` when that class exists. The baseline has only the unit suite available for this CI selection. Backend-dependent navigation tests remain available through `make apple-ui-test`; they were not part of the previous optimized CI selection.
- When the torrent crate exists, fixture seeder startup is required and its failure fails the job. Phase 0 local-file Playback Lab tests run without a seeder. Native test failures propagate normally. An optional ten-minute qualification is still explicit; requesting it without its fixture/project support fails rather than silently skipping.
- Tool versions and external actions are pinned. Cache roles, artifact names and logs are separated between jobs. Personal signing and generated build files remain ignored.

## Applying to the implementation PRs

After merging this PR, merge main into each feature branch in stack order. Those branches already contain older versions of the workflow, Makefile and command wrapper, so conflicts are possible. Preserve their additional Make targets, source files, project build phases and tests; keep this workflow's single combined invocation, native host tests, asynchronous simulator boot and required aggregate check. Do not replace an entire feature branch Makefile with the baseline one. Re-run all PR checks after resolving conflicts.

The three original quick wins were verified together on the product branch in [run 44115546737](https://github.com/supagroova/cue-tv/actions/runs/44115546737): 30 unit tests, 10 Playback Lab tests, both native suites and device compilation passed. That run is evidence for the prior implementation, not this new baseline-compatible PR. Simulator execution took 9m12s; runner queueing made elapsed workflow time 14m24s, so no guaranteed wall-clock saving is claimed.

## Verification and handoff

Local command-boundary tests cover baseline unit selection, combined unit/playback selection in one Xcode invocation, and Xcode failure propagation. The baseline test failed against the copied wrapper before the compatibility fix and passed afterwards. Run `python3 -m unittest discover -s tests/agent -v` and `bash -n scripts/agent/apple.sh` from the repository root. The baseline does not include the broader agent harness or its `make check`/`make staged-check` commands.

This PR is intended for user review and manual merge. It does not merge PRs #3–#7 or claim physical-device qualification. Check this PR's current GitHub Actions results before merging; feature-branch playback coverage must be verified again after applying it downstream.
