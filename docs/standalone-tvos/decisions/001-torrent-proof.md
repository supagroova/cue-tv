# ADR 001 — embedded torrent feasibility path

Status: accepted for the debug proof; production resource policy and player selection remain Phase 2 decisions.

Use librqbit at `f23bacdddcc5318caa813418b8eb1b1d7307faaa`, Rust 1.100.0, and the rust-tls feature set. The app links the static library and owns its session. The dependency builds for ARM64 tvOS and simulator after a narrow Apple socket-binding conditional fix, retained with upstream attribution. A libtorrent fallback has not been needed.

Use loopback HTTP to feed verified bytes to AVPlayer for the constrained H.264/AAC MP4 proof. Each session gets a random path and ephemeral loopback port. HEAD, full GET, open/closed/suffix ranges and 416 are covered. Multipart ranges are rejected. FileStream's piece verification and read-driven priority window are reused; the upstream lookahead is 32 MiB. Responses stream chunks rather than loading whole files. Existing response bodies must observe the session cancellation token; merely closing the listening socket left blocked readers alive, as the 64 MiB test demonstrated.

The C boundary uses numeric handles, version 1 JSON envelopes, matching snapshot session IDs and explicit string allocation/free. The Swift owner polls without waiting for native I/O on the main actor, and rejects obsolete generations. Native work uses two runtime workers and a 30-peer limit. Metadata/file descriptors and verified byte counts are exposed for diagnostics. Background/exit stops the proof session.

Tracker and DHT switches are independent. Deterministic host tests isolate HTTP tracker, UDP tracker and DHT lookup without initial peers or web seeds. A KRPC fixture provides a routing node and responds to `get_peers`; this establishes the client protocol path, not public DHT health or tvOS radio behavior. HTTPS tracker and public swarm qualification remain outstanding.

The Mac seeder is an owned test fixture, not a production backend. No Pi, Android API, paid debrid service or external-player handoff supplies playback. Physical independence remains deferred by the user's simulator-first direction. AVPlayer is not yet a production choice: MKV and the rest of the agreed media matrix must be evaluated without silently reducing scope.
