# Customer Insights harness adaptation

Source inspected: [livestorm/customer-insights at ab0f027](https://github.com/livestorm/customer-insights/tree/ab0f027479baa2eab23cd4d57d79302d8660e95c). Cue TV baseline is the standalone planning branch at `0b502b3661b910408ce33364c1ea7c902aa49d1b`. This is an adapted implementation, not a drop-in copy or runtime dependency on Livestorm.

## Capability disposition

| Source capability | Cue TV implementation / decision |
|---|---|
| Shared AGENTS/CLAUDE policy and skill aliases | AGENTS.md + CLAUDE import; canonical `.agents/skills`, Claude symlinks |
| how, why, recall | cue-investigate: code/test/history anchors, evidence levels, scoped handoff |
| architect, brainstorming, sharpen, in-your-own-words | cue-plan: consumer, intent, data/state shapes, alternatives and exact task contracts |
| writing-plans | cue-plan + task template, acceptance and interface ownership |
| executing-plans and task-start/task-done | cue-execute + Python evidence ledger; failures retained, no automatic completion claim |
| bug-fix / refactoring / feature / prototype / perf playbooks | cue-debug, cue-plan, cue-review and cue-device-proof; no web server or DB assumptions |
| tdd and audits | cue-test; meaningful seam tests plus harness self-tests; no arbitrary file-per-test or 85% gate |
| interrogate, thermo-nuclear review, blast-radius, merge-risk | cue-review lenses, concrete findings and proof; no paid model score or automatic review posting |
| no-comments and unslop | cue-review/document: remove duplication, preserve FFI/lifecycle rationale and licenses |
| benchmark-checklist | cue-device-proof: conditions, errors, repeatability, limiter, work performed, whole-user latency |
| developer-documentation | cue-document, link validation and operational/evidence distinction |
| correct / reflect | cue-improve; scoped evidence, structural fixes, behavioral checks; no transcript scraping |
| subagent-driven-development, swarm, arena | cue-parallel, explicit-use only; no forced delegation/model aliases; shared-device ownership |
| sixteen principles | Types/data/consumer/alternatives in plan; deletion/ownership in template; boundary/reader-load in review; run-twice, measurement and premise checks in test/debug/device skills |
| make CLI, docs validation and quality gates | Python/Make scripts and portable CI; explicit separate Xcode build/unit/UI gates |
| commit hooks and risky-command guards | Opt-in staged path guard. No claim that shell parsing secures arbitrary tools; no unverified provider hooks |
| session registration and provider switching | Durable tracked handoff plus ignored task evidence; current identity only, no transcript-copy mechanism |
| PR writing / ship | cue-ship + PR template; task-scoped staging and authorized destination; no auto-merge |
| TypeScript/Svelte rules, UI token audits, pg-boss idempotency, migration/DB gates | Not applicable. Swift/FFI/session/cache contracts replace the relevant ideas; no Node/Postgres dependency |
| Sentry triage, post-deploy revert, docs-drift/test-refactor cron, PR Lens, paid AI review workflows | Deferred: no corresponding deployed service, keys or authorized automation. Manual review/handoff covers present need |
| provider plugin settings, model tiers, organization permissions/MCP configuration | Not copied. Preserve user setup; discover only needed tools and verify access |
| customer data, analytics, integrations, API credentials | Excluded entirely |

## Attribution and scope

Concepts are adapted from the inspected repository's instructions, playbooks, skills and scripts. Its vendor notices identify Lauren Tan/pstack, obra/superpowers, michael-denyer/pstack-claude, warpdotdev/common-skills and Matt Pocock's TDD/engineering skills among upstream sources. Available license texts for those referenced materials are retained in `licenses/`. Newly written Python gates are Cue TV-specific; no TypeScript hook implementation or customer data was copied. Original Cue TV setup instructions remain in UPSTREAM-SETUP.md.

## Limits

The policy is discoverable repo text, not a security sandbox. Structural validation cannot prove test quality or temporal test-first work. Local hooks are opt-in; this checkout enables the staged-path guard using `make hooks`. The portable CI definition is authored but its remote execution must be observed before claiming success. Physical playback remains an unrun product gate. No new service account, global hook, paid reviewer or automatic publishing mechanism is activated.
