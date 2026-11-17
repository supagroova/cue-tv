# Task: meaningful deliverable

Execution mode: inline, unless explicitly authorized parallel work has independent write targets.

## Goal and acceptance

Link the PRD requirement/stage, consumer-visible outcome and decisive test. Distinguish automated checks from physical-device evidence.

## Scope and interfaces

- Files: exact owned source/test/document paths.
- Deletions: list removed paths/behavior, or none with a reason.
- Consumes: input types, callers, dependency assumptions.
- Produces: output types, errors and lifecycle states.
- Parse points: untrusted data validation and resource limits.
- Ownership: callbacks, buffers, threads, cache, cancellation and cleanup.
- Alternatives: relevant choices and why this one fits.

## Execution

1. Name the seam and defect; inspect existing coverage.
2. Reproduce the failure or establish the experiment baseline.
3. Implement the smallest coherent change.
4. Verify exact acceptance, negative cases and affected callers.
5. Review, record evidence and update the handoff.

## Evidence and recovery

Record commands, outcome, log path and tested content identity. For retries/stop/sync, run twice and verify no duplicate effect. State rollback and any data/schema consequences. Record pass, fail, blocked or not run; never complete from a checkbox alone.
