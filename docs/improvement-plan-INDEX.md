# Cellar Improvement Plan — Index & Execution Backlog

A single entry point to the planning docs produced during the planning loop, plus one merged, execution-ordered backlog combining bug fixes and features.

> **Status update (2026-06-10): Phase 1 is complete and merged.**
> - Grid cell-editor fixes (B1/B2/B13): [#58](https://github.com/MRL-00/cellar/pull/58)
> - Registry persist-before-mutate (B4): [#57](https://github.com/MRL-00/cellar/pull/57)
> - Result paging past the 500-row cap + B9: [#60](https://github.com/MRL-00/cellar/pull/60)
> - Settings persistence (Editor/Grid): [#59](https://github.com/MRL-00/cellar/pull/59)
>
> Next up: **Phase 2** (export, query cancellation, store/effect cleanup B6/B7/B8, rows_affected surfacing).

## The docs

| Doc | What's in it |
|---|---|
| [improvement-plan.md](improvement-plan.md) | The tiered roadmap (Tier 0 landmines → Tier 4 differentiators) + the user wishlist section. Start here for the "why/what". |
| [improvement-plan-top5.md](improvement-plan-top5.md) | Implementation-ready specs for the 5 highest-impact features (files, contract changes, acceptance criteria). |
| [improvement-plan-wishlist.md](improvement-plan-wishlist.md) | Implementation specs for the 3 user-requested wishlist items (CSV upsert import, dumps/restores, quick+advanced filter). |
| [improvement-plan-bugs.md](improvement-plan-bugs.md) | 13 correctness bugs (8 verified ✅, 5 suspected 🟡) with file:line, repro scenario, and fix. |

## The single most important finding

**The UI is far ahead of the backend.** Much of Cellar looks finished but is static stubs, and two classes of issue make it unsafe/unusable on real data today: (1) a hard 500-row cap on every query and table browse, and (2) correctness bugs in the editable grid (Escape commits instead of cancelling; NULL cells record phantom edits). Fixing those two classes is what turns Cellar from "impressive demo" into "usable daily on Postgres."

## Merged execution backlog (recommended order)

Each item links to its full spec. Bugs (B#) are in the bugs doc; features (#/W#/Tier) in the roadmap/spec docs.

### Phase 1 — Trust & safety (the grid must not lie or lose data)
1. **B1 + B2 + B13** — data-grid cell-editor fixes (Escape-commits, NULL phantom edit, double-commit). _Already flagged as a background task._ ✅ verified, contained, same file.
2. **B4** — persist-before-mutate in `ConnectionRegistry.save`/`delete` (state/disk divergence on I/O error). ✅
3. **Top 5 #1** — kill the 500-row cap → pagination + grid virtualization. Pulls in **B9** (SQL Server `continue`→`break`).
4. **Top 5 #3** — persist settings + make them take effect (frontend-only; unblocks the page-size knob for #1).

### Phase 2 — Feels finished
5. **Top 5 #4** — result export (CSV/JSON/SQL) + grid copy-as. Shares "fetch all rows" plumbing with #1.
6. **Top 5 #2** — query cancellation (`Driver::cancel_query` + `pg_cancel_backend`).
7. **B6 + B7 + B8** — frontend store/effect cleanup: clear messages/notices on tab close, fix sticky dirty flag, stale-guard the EXPLAIN/table-load async results.
8. **rows_affected** surfacing (Tier 0 #3) — the field already exists on `QueryResult`, just always `None`.

### Phase 3 — Competitive parity
9. **Top 5 #5** — schema-aware autocomplete (the #1 felt gap vs DataGrip).
10. **W3** — quick + advanced filter coexistence (mostly grid state; rides with #1).
11. **B3** — SQL Server `Contains` LIKE-metacharacter escaping (silent wrong results). ✅
12. **Tier 2** — FK navigation in grid · generate SELECT/INSERT/UPDATE/DELETE · type-aware cell editors.

### Phase 4 — Breadth & wishlist
13. **W1** — CSV upsert/update import (reuses `cellar-diff`; the one backend add is an `ON CONFLICT` variant).
14. **Tier 3** — SQLite driver (low-effort, high-delight) · SQL Server EXPLAIN + commit-edits · SSH tunneling.
15. **B5** — confirm + fix Postgres OID/TIMETZ decode (needs a live catalog-query repro first). 🟡
16. **W2** — dumps/restores ("Data movement" workstream).

### Phase 5 — Differentiators
17. ER diagram view · read-only/prod guardrails · crash recovery/session restore · plugin runtime + exporters · DataGrip/DBeaver import · demo database.

## Open decisions for the maintainer
- **Build vs issues:** these can be cut straight into GitHub issues (one per B#/#/W#) or picked up directly for implementation.
- **B5/B8/B10/B13 repros:** require running the app against a live Postgres/SQL Server to demonstrate; worth a containerized test harness.
- **Driver priority:** SQLite (easy, broad appeal) vs finishing SQL Server (commit-edits + EXPLAIN) — depends on target users.
