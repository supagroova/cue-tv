# Standalone tvOS handoff

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
