# Device proof report

Status: not run

## Identity

App revision/content digest, dependency pins, Xcode/Rust/SDK versions, device model, actual tvOS build, signing configuration identifier without secrets, fixture checksum and usage permission.

## Conditions

Wi-Fi conditions, seeder/discovery protocol, other discovery paths disabled for isolated tests, cache cold/warm, selected file and total bytes, player/profile, debugger attached/detached, Pi/Mac dependency state. Separate controlled swarm and public swarm results.

## Observations

Record first frame and verified bytes at that instant, completion fraction, uncached seek timings, cancellation/teardown, memory, allocated disk, reserve and caps, error counts, audio/video behavior and repeat count. Record run range/median where appropriate, not only a best run. Verify the measured work actually happened.

## Verdict

Compare observations to the specific plan gate. Identify fail/inconclusive/not-run cases, sanitized evidence paths and next decision. An HTTPS/web-seed stream, simulator or fully downloaded local file does not prove direct peer streaming. A successful controlled seeder run alone does not establish production independence.
