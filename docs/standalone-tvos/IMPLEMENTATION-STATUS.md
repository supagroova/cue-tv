# Implementation status

The user selected simulator-first development and deferred physical Apple TV pairing. Hardware qualifications remain unverified; do not infer them from host or simulator evidence.

| Phase | Implementation/evidence | Remaining |
| --- | --- | --- |
| 0 | PR #4: generated media, decoder lab, unit and remote-driven simulator tests | Physical signing/install/launch qualification |
| 1 | PR #5: embedded native engine, real peer transfer, magnet metadata, file selection, isolated HTTP/UDP/DHT fixtures, versioned C ABI, range adapter, cancellation regression, ten-minute simulator playback starting before completion | HTTPS/public swarm, stress/error cases, physical independence |
| 2 | Native storage admission and write-time reserve guards; pinned VLCKit candidate builds for tvOS and plays/stops an owned H.264/AAC MKV in a simulator UI test | Player matrix/selection, session coordinator, app-wide cache ownership/cleanup and measured resource policy |
| 3–5 | Not implemented | Core, standalone journey, state/sync/recovery |
| 6–7 | Not implemented | Qualification and reproducible personal release/handover |

PR #2 was merged by the user into the planning branch. Its merge commit is included downstream, and Phase 0 now targets that planning branch. No implementation PR has been merged by the agent.

See PHASE-0.md, PHASE-1.md and decisions/001-torrent-proof.md for commands, measured evidence and limits. The original PRD and implementation plan still define the complete scope; this status does not replace their acceptance criteria.

The repository was transferred to public supagroova/cue-tv by its owner. The local origin now uses that canonical address. Earlier GitHub Actions jobs failed before starting because of billing/spending limits; after the transfer, harness CI passes and build jobs start again. The complete stack is not yet verified green.
