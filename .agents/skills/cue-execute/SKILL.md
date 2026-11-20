---
name: cue-execute
description: Execute a Cue TV plan task with durable evidence, scope control and resumption after interruption.
---

# Execute

Read the task and linked requirements; inspect current state before editing. Keep a single task's source/test/document ownership clear. Use scripts/agent/task.py start with a unique task ID, then work in observable vertical slices. Existing user changes are not yours to stage or revert.

Run the matching tests through task.py verify. Failure is retained; repair and rerun the affected check. A green command does not prove the whole task: compare to acceptance, review lifecycle/FFI implications and collect device evidence when required. After edits, invalidate stale evidence. Finish with the exact outcome, changed files, remaining concerns and next task in the handoff. Do not auto-commit, publish or install merely because the helper returned success.
