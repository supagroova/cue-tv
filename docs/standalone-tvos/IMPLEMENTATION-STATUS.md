# Implementation status

The user selected simulator-first development and deferred physical Apple TV pairing. Hardware qualifications remain unverified; do not infer them from host or simulator evidence.

| Phase | Implementation/evidence | Remaining |
| --- | --- | --- |
| 0 | PR #4: generated media, decoder lab, unit and remote-driven simulator tests | Physical signing/install/launch qualification |
| 1 | PR #5: embedded native engine, real peer transfer, magnet metadata, file selection, isolated HTTP/UDP/DHT fixtures, versioned C ABI, range adapter, cancellation regression, ten-minute simulator playback starting before completion | HTTPS/public swarm, stress/error cases, physical independence |
| 2 | PR #6: native cache-root admission/write guards and acknowledged cleanup; selected pinned VLCKit after MPVKit comparison; MP4/MKV, HEVC, track and subtitle probes; serialized coordinator and startup/seek/stall deadlines; manual distant torrent seek and simulator resource samples | Remaining media/error paths, realistic resource budgets, production adapter wiring, physical decode/resource qualification |
| 3 | PR #7: pinned native Core environment and owned context/action loop; isolated memory storage, bounded HTTP, joined cancellation, FIFO effects and injectable clock; model/storage tests and tvOS cross-compilation | More model operations, Swift/FFI service, persistent guest state, protected credentials and account lifecycle |
| 4–5 | Not implemented | Standalone journey, state/sync/recovery |
| 6–7 | Not implemented | Qualification and reproducible personal release/handover |

PR #2 was merged by the user into the planning branch. Its merge commit is included downstream, and Phase 0 now targets that planning branch. No implementation PR has been merged by the agent.

See PHASE-0.md, PHASE-1.md and decisions/001-torrent-proof.md for commands, measured evidence and limits. The original PRD and implementation plan still define the complete scope; this status does not replace their acceptance criteria.

The repository was transferred to public supagroova/cue-tv by its owner. The local origin now uses that canonical address. Earlier GitHub Actions jobs failed before starting because of billing/spending limits; after the transfer, harness CI passes and build jobs start again. The complete stack is not yet verified green.
