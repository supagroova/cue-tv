---
name: cue-review
description: Review Cue TV changes for correctness, security, maintainability, blast radius and merge readiness.
---

# Review

Establish intended behavior, actual base/head and owned diff. A stacked branch uses its real base, not main automatically. Review-only means report findings, no edits or external writes.

Inspect correctness/security (untrusted paths, ranges, secrets, ABI lifetimes), structure (ownership, duplicated state, adapter boundaries), verification (real vs mocked/device evidence), and rollback (persistence/signing/dependencies). Trace callers and serialization boundaries beyond grep. Identify the one fact safety relies on and prove it with real code where feasible. Keep speculative risks labeled unproven; cite concrete failure paths and changed locations for defects.

For refactors preserve behavior and assertions, migrate all callers then remove obsolete paths. Remove comments that paraphrase code; retain lifecycle, FFI, license and non-obvious rationale. Classify findings as act/consider/cleared with evidence, not severity inflation or an arbitrary file-size score. Evaluate alignment, impact, verification, rollback and error detection; do not invent numerical risk precision. Delegated lenses require explicit authorization; otherwise perform them inline and say so. No independent-review claim for self-review.
