---
name: cue-plan
description: Design Cue TV features and turn accepted requirements into executable task plans with ownership and evidence gates.
---

# Plan

Reuse the PRD and staged plan. Separate a bug, bounded change and architectural decision. For an unresolved design, name the consumer and data/state shape; compare viable alternatives, including deleting/reusing existing code. Inspect dependency APIs before naming signatures. Do not force a new product approval for settled requirements.

Use docs/agent-harness/templates/task.md for the next implementable unit. Name exact owned files, consumed/produced interfaces, parse points, buffer/thread ownership, cancellation, deletions, tests and rollback. Include run-twice tests for retry/stop/sync and state migration. Resolve near-term signatures now; leave unproven later engine decisions as explicit gates rather than invented APIs. Keep UI work behind a real byte/playback proof. Record only meaningful decisions, not a transcript of prospective code.
