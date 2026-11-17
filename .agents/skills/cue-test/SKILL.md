---
name: cue-test
description: Write or review meaningful Swift, Rust and harness behavior tests, especially parsing, cancellation and FFI boundaries.
---

# Test

Name the observable seam and the distinct defect before adding a test. Read existing tests; strengthen them rather than duplicate coverage. Use independently specified literals/fixtures for expected values. Avoid tests that merely restate implementation, types, mocks or generated wording.

For nontrivial behavior use red → minimum green → refactor. Confirm the red cause and demonstrate sensitivity to the relevant defect when not already established by reproduction. Test sequential slices rather than bulk imagined interfaces. Mock external network/clock/device boundaries, not internal implementation. Documentation or trivial presentation changes need relevant review, not artificial unit tests.

Prioritize invalid hashes/indexes/paths/ranges, partial reads and EOF, stale callbacks, cancel during blocked reads, repeated stop, account isolation, quota exhaustion and missing cache. Simulator tests cannot establish radio/peer/decoder/suspension behavior. Record unavailable device tests as not run; never skip or weaken assertions to satisfy a gate.
