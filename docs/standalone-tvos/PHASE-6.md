# Phase 6 — full-app qualification

Status: preparation in progress; **not qualified and not merge-ready**. This branch
stacks on `codex/tvos-phase-5`. Starting the matrix does not waive unresolved gates
in phases 0–5. The [PRD](PRD.md) and [implementation plan](IMPLEMENTATION-PLAN.md#stage-6--qualify-the-full-app-on-a2737)
remain authoritative. Routine CI proves builds and bounded automated checks;
physical qualification is manual and must not become a long per-push CI job.

## Identity and entry requirements

The user reported Apple TV 4K A2737, 64 GB, tvOS 27.2. These details have not been
read from the device. Current local compilation/simulator evidence uses Xcode 26.6
and SDK/runtime 26.5. Do not infer device compatibility from that evidence.
Pairing and physical testing were deferred by the user.

Before recording a run, use the [device-proof report](../agent-harness/templates/device-proof.md)
and capture:

- Actual device model, storage capacity, tvOS version/build and available space.
- Git commit, clean/dirty content digest, app build/configuration and signing identity
  identifier without credentials; debugger attached/detached state.
- Pinned dependency revisions and toolchain versions from the tested checkout.
- Fixture provenance/permission, file and torrent checksums, selected file, codecs,
  container, resolution, audio/subtitle tracks and full file size.
- Controlled seeder address/protocol, Wi-Fi conditions, discovery paths enabled,
  cold/warm cache state and whether any HTTP media/web-seed path is possible.
- Numeric memory and allocated-cache ceilings established from phase 2 on the
  actual hardware. These are currently **unmeasured**, not implicit passes.

The present 4 GiB cache limit and 1 GiB free-space reserve are provisional policy
values, not measured safe memory/disk budgets. Never fill the user's disk to test
failure: use the quota fault boundary and retain enough reserve for system use.

## Mandatory matrix

Every row below is **not run on A2737**. Host/simulator results listed in earlier
phase reports are supporting evidence only. A missing measurement is inconclusive,
not zero. Record each failure and retry separately, including the intervening fix.

| ID | Required experiment | Passing evidence |
| --- | --- | --- |
| Q01 | Ten cold-cache starts from controlled peers | At least 9/10 first frames within 20 s; record every time, verified bytes at first frame and full size; playback begins before completion |
| Q02 | Ten seeks into genuinely uncached ranges | At least 9/10 resume within 10 s; record target/range, prior piece availability and resumed decoded frame |
| Q03 | Three independent 60-minute baseline-media sessions | No crash, wrong file or sustained A/V drift; timestamped drift observations, memory high-water and allocated-cache measurements below recorded ceilings |
| Q04 | Twenty source switches | No orphaned transfer/reader/decoder and no unbounded resource growth; retain per-switch session identity and post-stop resource observations |
| Q05 | Pause, Back and Cancel during stalled discovery/reads | User-visible response within 1 s; joined teardown, no late callback reviving the old session; test each control separately |
| Q06 | Zero peers and unavailable tracker | Bounded recoverable error; retry after restoring peers/tracker succeeds without replacement-owner leakage |
| Q07 | Network loss and recovery | Interrupt metadata and active transfer separately; bounded error/retry, retained account state and correct selected file on recovery |
| Q08 | Invalid metadata, paths and file indexes | Reject each class without traversal, crash, incorrect-file playback or leaked resources |
| Q09 | Unsupported codec/container/audio | Clear bounded failure or documented supported fallback; no false playback success; record actual format/decoder |
| Q10 | Injected write exhaustion and low-space admission | Distinct recoverable errors, bounded writes/reserve protection and successful retry after fault removal |
| Q11 | Disposable media-cache purge | Purge between sessions and during an active session separately; account/Keychain/journal intact; controlled peer reacquisition and recovery without wrong bytes |
| Q12 | Ten resume cycles including suspension and relaunch | Position loss at most 15 s; include Home, sleep, interruption and abrupt process termination, with cause and last durable checkpoint recorded |
| Q13 | Account isolation and failed-sync recovery | Guest/account transitions, rejected auth, failed storage, cancellation/cleanup retry and sign-out preserve ownership; no old-account data or callbacks in replacement account |
| Q14 | Full guest journey with Pi off and development Mac disconnected | Catalog → chosen episode/file → raw peer playback → controls → resume; no runtime companion dependency |
| Q15 | Full connected Stremio journey under the same independence conditions | Permitted account sign-in/restoration, browsing/library, chosen source and raw peer playback; controlled fixture success does not qualify live service |
| Q16 | Siri Remote focus and accessibility | Review all main flows, errors/retry, dialogs, source/episode lists, player controls and return focus; readable labels and usable accessibility navigation |
| Q17 | Same-identity upgrade | Install previous then candidate signed build; preserve account/resume/library state and launch with debugger detached; record both revisions/profile identifiers |
| Q18 | Release configuration audit | No lab/fixture controls, broad debug transport exceptions or external-player dependency; inspect the built bundle and exercise release startup/playback |

Run isolated HTTP(S)-tracker, UDP-tracker and DHT discovery cases with alternate
paths disabled, as required by the earlier feasibility gates. Keep public-swarm
observations separate from controlled thresholds. A local completed file, HTTPS
media stream or debrid response is not evidence of direct torrent streaming.

For Q14/Q15 the development Mac must not be the only available controlled seeder.
Use an independently hosted permitted fixture or clearly identify a public swarm;
turning the Mac off while it is the only peer would test zero peers instead of
application independence. Do not change the required standalone app architecture.

## Recording and verdict

Keep one report per tested revision/device/fixture configuration. Each report must
link its raw observations and sanitized artifacts, list all attempted runs, and
identify clock origins: start means the user's selection; first frame/resumed frame
means decoded visible output, not engine admission or downloaded-byte counters.
Record median/range and sample count as well as threshold success counts. For Q03
include periodic memory/cache samples and timestamped A/V observations, not merely
an elapsed timer. For Q04 verify resource release after each switch.

Use `pass`, `fail`, `inconclusive` or `not run` for each case. Do not replace a failed
record with a successful retry. An amended product limitation requires the user's
explicit PRD decision; this matrix cannot silently redefine acceptance. Commit only
sanitized evidence summaries/checksums, never credentials, tracker passkeys,
private viewing history, signed provisioning material or raw account snapshots.

Release qualification requires every mandatory row and inherited phase gate to
have adequate evidence. Green CI alone does not meet this condition.

## Current limitations and next work

- Physical identity, signing, launch, network/decoder performance and lifecycle
  behavior remain unverified. 4K/HDR/audio capability must not be advertised from
  simulator or framework-support claims.
- Live Stremio account/add-on qualification and some phase 5 failure/UI journeys
  remain open; see [phase 5](PHASE-5.md) and the latest [handoff](../agent-harness/HANDOFF.md).
- `apple-tv/project.yml` currently grants `NSAllowsArbitraryLoads` globally. Q18
  cannot pass until release transport settings are narrowed and the resulting
  release bundle plus required playback transports are checked.
- Hardware-derived memory/cache ceilings, active cache-purge recovery, sustained
  playback and independent guest/account journeys have no physical evidence yet.
- No measured row is marked passed here. Next implementation work includes release
  hardening and qualification tooling; hardware runs remain deferred.
