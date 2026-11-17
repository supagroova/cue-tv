# Standalone tvOS handoff

The bootstrap notes below are historical. Current product progress is tracked in [implementation status](../standalone-tvos/IMPLEMENTATION-STATUS.md).

## Goal and state

Direct torrent playback on Apple TV; personal sideloading; Stremio-only v1. PRD, research and staged plan are in `docs/standalone-tvos/`. Source matches the extracted upstream baseline. No torrent engine, Stremio Core bridge or in-app player has been implemented. No physical-device playback is verified.

## Next work

Implementation stage 0: inspect Xcode/device compatibility, pair with the user's participation, configure a personal app identity and launch a debug Playback Lab. Then stage 1: embed and prove raw torrent transfer/playback before catalogue integration.

## Environment

A2737 / 64 GB / tvOS 27.2 is user-reported; verify OS/build during pairing. Local folder is a full checkout on `codex/tvos-agent-harness`, based on the saved planning branch. GitHub CLI authentication works over HTTPS using its credential helper; SSH authentication did not. The earlier extracted snapshot is retained in the sibling `cue-tv-snapshot-backup` directory. No signing configuration should enter source control.

## Evidence

Prior app baseline: unsigned build and five unit tests passed, as recorded in `docs/standalone-tvos/LOCAL-BUILD.md`. These do not prove networking/playback. Harness verification is recorded in `VERIFICATION.md`; rerun checks after subsequent edits. Device gates remain not run.

## Resume checklist

Read the relevant plan stage, inspect current files and branch when available, check task logs against current content, preserve completed work and pending authorization, then execute the next uncompleted task. Do not restart the investigation or re-ask confirmed product choices.

## Apple CI speed follow-up — 2026-11-17

Branch `codex/apple-ci-speed` is based on `614b74d`. The workflow now runs native/device and simulator jobs independently, preserves the aggregate check, and combines unit/short UI tests in one Xcode build. See [Apple CI execution](APPLE-CI.md) for exact commands, baseline timings, verification and limits. Local combined validation passed 25 unit and 10 short UI tests; the remote speedup is not yet measured. Next: integrate the reviewed commit, run the workflow and compare both cold and warm timings. Physical-device evidence is unchanged.


CI follow-up: warm role-cache run `44113416860` passed all checks in 9m02s queue-inclusive, with the simulator lane taking 8m52s. An ordering-only experiment now defers simulator boot until after cache/tool setup; local 12-test harness and actionlint pass, remote timing remains to be measured. See [Apple CI measurements](APPLE-CI.md).
