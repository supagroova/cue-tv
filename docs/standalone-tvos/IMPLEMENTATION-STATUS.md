# Implementation status

The user selected simulator-first development and deferred physical Apple TV pairing. Hardware qualifications remain unverified; do not infer them from host or simulator evidence.

| Phase | Implementation/evidence | Remaining |
| --- | --- | --- |
| 0 | PR #4: generated media, decoder lab, unit and remote-driven simulator tests | Physical signing/install/launch qualification |
| 1 | PR #5: embedded native engine, real peer transfer, magnet metadata, file selection, isolated HTTP/UDP/DHT fixtures, versioned C ABI, range adapter, cancellation regression, ten-minute simulator playback starting before completion | HTTPS/public swarm, stress/error cases, physical independence |
| 2 | PR #6: native cache-root admission/write guards and acknowledged cleanup; selected pinned VLCKit after MPVKit comparison; MP4/MKV, HEVC, track and subtitle probes; serialized coordinator and startup/seek/stall deadlines; manual distant torrent seek and simulator resource samples | Remaining media/error paths, realistic resource budgets, additional production failure paths, physical decode/resource qualification |
| 3 | PR #7: pinned Core environment and Swift/C/Rust boundaries; encrypted identity-scoped storage, Keychain account selection, login/restore primitives; bounded HTTP, joined cancellation, paged catalogs/search/metadata/source handoffs | Retained model/cache bounds, remaining library/player operations, complete authentication/cancellation/crash-recovery qualification, physical Core proof |
| 4 | PR #8: native guest/account Home, sign-in/cancel/sign-out/recovery, catalog search, season/episode filtering, revision-bound row selection, source/file selection, shared torrent/HTTPS player with deadlines, controls and bounded stream-provided and separately discovered add-on subtitles; local TLS, encrypted restoration, remote Back/focus and complete owned catalog-to-peer playback fixtures | Hash-dependent subtitle hints, artwork, search/season-picker/movie UI coverage, permitted live add-on validation, live authentication and process-crash cleanup |
| 5 | PR #13: durable progress and Resume/Restart, library/watched actions, encrypted bounded sync journal, same-account refresh, local diagnostics; real-peer simulator and synthetic account UI evidence; lost Close receipt and Start/Retire retry fixes | Full UI timeout/cleanup-failure journeys, live accounts, abrupt interruption, physical lifecycle/privacy qualification; phase remains incomplete |
| 6 | Qualification matrix and explicit evidence protocol prepared in PHASE-6.md; no physical results yet | Release hardening, measurements and every mandatory A2737 qualification row |
| 7 | Current local build guide and candidate/handover protocol prepared in PHASE-7.md | Final binary/source manifest, notices, in-app help, signed install/upgrade/renewal/removal and independent physical use |

PR #2 was merged by the user into the planning branch. Its merge commit is included downstream, and Phase 0 now targets that planning branch. No implementation PR has been merged by the agent.

See PHASE-0.md through PHASE-7.md and the decisions directory for commands, measured evidence and limits. The original PRD and implementation plan still define the complete scope; this status does not replace their acceptance criteria.

The repository was transferred to public supagroova/cue-tv by its owner. The local origin now uses that canonical address. Earlier GitHub Actions jobs failed before starting because of billing/spending limits; after the transfer, harness CI passes and build jobs start again. The complete stack is not yet verified green.

PR #7 CI passed after retrying a simulator job abandoned by GitHub runner capacity. This does not establish phase-three feature completion. Recent PR #8 and CI-optimization runs also encountered macOS ARM runner shortages; execution failures and queue failures are recorded separately. The full stack remains incomplete, with phase 5 in PR #13 and no phase 6–7 PR yet.
