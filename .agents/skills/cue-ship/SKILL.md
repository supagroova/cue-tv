---
name: cue-ship
description: Prepare an authorized Cue TV commit, branch backup or PR with exact verification and scoped staging.
---

# Ship

Inspect repository state and actual destination. A snapshot is not a Git checkout: use authorized connector writes based on an existing upstream tree or obtain a real clone; never delete upstream files absent from the snapshot. Select only task-owned files. Exclude generated projects, signing, profiles, credentials, logs and media. Run make check, applicable app/native/device checks and make staged-check in a real checkout. Review evidence for the current content, not an earlier revision.

Commit a coherent change with an accurate message and push only the authorized work branch. Do not force-push, merge, open a PR or post messages unless that action is in scope. For a PR describe the user-visible problem/result, scope, consequential tradeoffs, blast radius and measured verification. Keep failures/not-run checks explicit. After upload compare remote blob hashes with the intended local files. Preserve unrelated changes and main.
