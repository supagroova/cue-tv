---
name: cue-debug
description: Investigate and fix Cue TV regressions with reproducible evidence at the failing runtime boundary.
---

# Debug

Reproduce at the reported surface. Separate torrent discovery, verified-byte delivery, byte-range serving and decoder controls using a known local-file control. Record hypotheses and eliminate them with measurements. If the report cannot reproduce, say so and test the mechanism without claiming the original is fixed.

Write or strengthen a regression test at the observable boundary; confirm red for the intended reason, implement the cause-level fix and verify green at the original surface. For hardware-only behavior retain a reproducible device procedure and narrow host checks where useful. After two fixes based on the same premise fail, revisit that premise instead of adding fallback layers. Remove only your own disproven changes, preserve tests/user edits, review analogous callers and record remaining uncertainty.
