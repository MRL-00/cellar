# Cellar — Next Steps to Become the Best Database Client

_Audit date: 2026-06-09. Based on a full sweep of the desktop app, Rust drivers, and SPEC.md._

The AI workflow is being built out in a separate loop. This plan covers **everything else**: broken/stubbed menu items, correctness bugs, production gaps, and the table-stakes features that DataGrip/TablePlus/DBeaver have and Cellar doesn't yet.

The audit found a recurring theme: **the UI is far ahead of the backend.** Many surfaces _look_ finished but are static stubs (`title="… not wired yet"`). The biggest credibility wins are turning that existing chrome into real functionality, plus fixing two correctness landmines that make Cellar unsafe on real databases.

---

## Tier 0 — Correctness landmines (do first; these make Cellar unsafe/unusable on real data)

1. **The 500-row hard cap on everything.** `DEFAULT_MAX_ROWS = 500` in the Postgres query path and `MAX_TABLE_BROWSE_ROWS = 500` for browsing. No serious client can ship with this. Needs real keyset/offset pagination + grid virtualization wired end-to-end, with a footer showing "showing N of M" and a "load more / fetch all" control.
   - Files: `crates/cellar-drivers/postgres/src/query.rs:55`, `table_browse.rs`, `apps/desktop/src/components/BottomPanel.tsx`.
2. **No query cancellation.** A runaway query locks the session with no escape. Add `cancel_query` to the `Driver` trait; Postgres via `pg_cancel_backend` on a second connection. This is table-stakes.
   - Files: `crates/cellar-core/src/driver.rs:109`.
3. **`rows_affected` is always `None`.** Run an `UPDATE`/`DELETE`/`INSERT` in the editor and Cellar can't tell you how many rows changed. Surface it in the Messages tab.
   - Files: all drivers' `query.rs`; `BottomPanel.tsx:313` ("rows-affected not surfaced yet").
4. **SQL Server cross-database `USE [db];` is stateful and unsafe** — concurrent queries on the single Mutex-guarded client see the wrong database. Adopt the Postgres sibling-pool-per-database approach.
   - Files: `crates/cellar-drivers/sqlserver/src/query.rs:21`, `connect.rs:17`.
5. **SQL Server SSL `Prefer`/`Require` silently trust any cert** (`trust_cert()`), so "Require" gives no real verification. Fix the mapping so only `Disable`/`Prefer` skip validation.
   - Files: `crates/cellar-drivers/sqlserver/src/connect.rs:126`.

## Tier 1 — Wire the stubs that already have UI (high ROI, low risk)

These are buttons/panels that already exist and just need a backend. Fastest path to "feels finished."

1. **Persist settings.** Almost the entire Settings modal is `StaticSegment`/`readOnly` with no `onChange` — Editor, Grid, General, History, Keymap, Connections panels don't save anything. Wire them to `~/.cellar/settings.json` and make them actually affect the editor/grid.
   - Files: `settingsWorkspacePanels.tsx`, `settingsDataPanels.tsx`, `settingsSystemPanels.tsx`.
2. **Result/grid export.** "Export not implemented yet" button in the bottom panel; no CSV/JSON/TSV/SQL export anywhere. Ship the first-party exporters the SPEC already promises (CSV, TSV, JSON, SQL INSERTs) + "copy as INSERT" and "copy as TSV/CSV" in the grid.
   - Files: `BottomPanel.tsx:148`, grid copy handlers.
3. **SQL formatting.** Format button is disabled pending `cellar-sql`, which is an empty placeholder crate. Build (or vendor) a Postgres-dialect formatter and wire format-on-demand + format-on-save.
   - Files: `SqlEditor.tsx:152`, `crates/cellar-sql`.
4. **Query bookmarks / saved queries.** Bookmark button disabled. Saved/named queries is one of the most-requested features missing here.
   - Files: `SqlEditor.tsx:184`.
5. **Keymap rebinding.** Preset selector is static; rebind buttons disabled. Wire to a real keymap store with VS Code / DataGrip / Vim presets (SPEC §6.8).
6. **Migration export from the commit modal.** "Migration export is not wired yet" — generate a `.sql` migration file from pending grid changes.
   - Files: `CommitModal.tsx:258`.
7. **Live external links + updater.** Docs/GitHub/Changelog/About links are all disabled no-ops across EmptyState and Settings; "Check for updates" is dead. Wire `tauri-plugin-shell` opener + the Tauri updater (also needs `tauri.conf.json` CSP, currently `null`).
8. **Replace hardcoded placeholder stats** in Privacy/History settings (e.g. "23,418 queries · 14.2 MB") with real counts — currently lies to the user.

## Tier 2 — Core editor/grid features that competitors have and we don't

1. **Schema-aware autocomplete** in the SQL editor (tables post-FROM, columns post-WHERE, functions). The editor exists; completion from live introspection does not. This is the #1 thing people feel missing vs DataGrip.
2. **Foreign-key navigation in the grid** — click an FK cell to open the referenced row in a new tab. SPEC §6.5; FK metadata is already introspected.
3. **Generate SQL from schema tree** — right-click table → generate SELECT/INSERT/UPDATE/DELETE (only "SELECT *" exists today).
4. **Type-aware cell editors + Excel paste** — date pickers, enum dropdowns, NULL toggle, paste a block from Excel/CSV into the grid.
5. **Tab split views** — SPEC promises drag-to-split (two queries / query+table side by side); only single tabs exist.
6. **Multi-statement execution with correlated result tabs** — run a file, get one result set per statement.
7. **Open structure / DDL view** for a table (columns, indexes, constraints, "Copy CREATE TABLE").

## Tier 3 — Broaden the engine matrix (Cellar is really a Postgres client today)

1. **MySQL + SQLite drivers** — declared in the `Engine` enum but no crate exists; `cellar-drivers/src/lib.rs` is a one-line placeholder. SQLite especially is low-effort, high-delight (local files, demo DB).
2. **SQL Server: add EXPLAIN + commit-edits path.** EXPLAIN returns "not available yet"; there's no `commit_table_changes` for SQL Server, so grid editing is Postgres-only. `cellar-diff` only emits Postgres dialect.
3. **Firestore: at least basic query execution** (currently every query/EXPLAIN/sort/filter returns "not supported yet"; it's browse-only).
4. **SSH tunneling + mTLS/custom CA** — `ConnectionConfig` has no SSH fields at all; the SSH tab in the dialog is UI-only. This is a hard requirement for most companies' prod databases.

## Tier 4 — Differentiators that push toward "best in class"

1. **ER diagram view** (SPEC post-1.0) — schema visualization is a marquee DataGrip/DBeaver feature.
2. **Connection-wide read-only mode + prod guardrails** — SPEC §6.1 promises a read-only toggle and confirmation before unguarded `DELETE`/`UPDATE`/DDL on `prod`-tagged connections. The prod badge exists; the safety behavior doesn't.
3. **Crash recovery / session restore** — restore open tabs, unsaved queries, and pending edits across restarts (SPEC §7); persist pending edits to local SQLite.
4. **Plugin runtime** — `cellar-plugin-host` is an empty placeholder; the whole extensibility principle is unrealized. Exporters are the easiest first plugin type.
5. **DataGrip/DBeaver/TablePlus import** — "Import from DataGrip/DBeaver" button is disabled; importing existing connections is a huge adoption lever.
6. **Demo database** — disabled "Connect to demo database" button; a one-click sample DB makes first-run delightful.

---

## Suggested sequencing

- **Sprint A (safety + polish):** Tier 0 #1–3 (pagination, cancellation, rows-affected) + Tier 1 #1–2 (settings persistence, export). This alone moves Cellar from "demo" to "usable daily on Postgres."
- **Sprint B (feel finished):** remaining Tier 1 (formatting, bookmarks, keymap, links/updater) + Tier 2 #1–3 (autocomplete, FK nav, generate SQL).
- **Sprint C (breadth):** SQLite driver, SQL Server EXPLAIN/commit, SSH tunneling.
- **Sprint D (differentiate):** ER diagram, read-only/prod guardrails, plugin runtime + exporters.

## Wishlist (user-requested; not yet slotted into a tier)

These come from real day-to-day pain with Sequel Pro / TablePlus and are strong differentiators. None are covered by the tiers above — slot them in as noted.

1. **Stable, elegant CSV upsert/update import — _"the Sequel Pro import"_.** Pick a CSV, map its columns to table columns, **choose a primary key (or unique column set) to match on**, then update all selected fields with the CSV values for matched rows (and optionally insert unmatched rows). Sequel Pro did this far better than TablePlus. This is more than the "Excel paste into grid" idea in Tier 2 #4 — it's a dedicated import wizard with column mapping, match-key selection, update-vs-insert-vs-upsert mode, a dry-run preview of affected rows, and a single transactional commit. Builds naturally on the existing `cellar-diff` pending-changes → transactional-SQL engine. **Suggested home: Tier 2 (core grid/data feature), high priority.**
   - Touches: new import wizard modal; `cellar-diff` (reuse plan builder for UPDATE/INSERT by match key); commit path in `postgres/src/query.rs:commit_table_changes`.

2. **Better / faster dumps and restores.** First-class schema + data dump (whole DB, selected schemas, or selected tables) and restore, with a progress UI and sensible defaults — faster and less fiddly than shelling out to `pg_dump` by hand. Note: SPEC §2 lists "backup orchestration" as a non-goal, so scope this as **developer-convenience dump/restore** (export a table or DB to a `.sql`/archive, restore from one), not cluster/backup management. Streaming so big dumps don't materialize in memory. **Suggested home: Tier 3 (engine-level capability) or a new "Data movement" area, medium priority.**
   - Touches: new driver capability (wrap `pg_dump`/`pg_restore` where present, or native COPY-based export); progress events over IPC; UI in connection/table context menus.

3. **Better coexistence of quick filter + advanced filtering.** Let a fast quick-filter (e.g. type an ID / free-text) live alongside structured advanced filters (the type-aware `=`/`LIKE`/`IS NULL` clauses already in `TableFilterClause`) without one clobbering the other. TablePlus does this passably but it "could do with love" — make the quick filter an additive layer over advanced filters, both visible, independently clearable, with a clear indicator of what's active. **Suggested home: Tier 2 (grid UX), medium priority.**
   - Touches: `@cellar/data-grid` filter bar; `BottomPanel.tsx`; the `TableBrowseRequest.filters` plumbing (`crates/cellar-core/src/query.rs:200`) already supports multiple clauses, so this is largely a UI/state-composition problem.

## Top 5 if you only do five things

1. Kill the 500-row cap (pagination + virtualization).
2. Query cancellation.
3. Persist settings + make them take effect.
4. Result export (CSV/JSON/SQL).
5. Schema-aware autocomplete.
